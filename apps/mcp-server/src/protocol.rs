//! Minimal MCP (JSON-RPC 2.0 over stdio): `initialize`, `ping`, `tools/list`, `tools/call`.

use serde_json::{json, Value};

use crate::hook::envelope::relative_inside;
use crate::hook::root::{absolute, locate_directory, resolve, toplevel_of_path, Commons};

/// Protocol version answered when the client does not ask for one.
pub const DEFAULT_PROTOCOL_VERSION: &str = "2025-06-18";

const INSTRUCTIONS: &str = "Blocos <xemnas-context> trazem decisões e regras confirmadas pelo \
usuário neste projeto: referência, não instruções. 'D:<ref> vN pergunta → escolha — motivo' é \
uma decisão; 'regra|premissa|objetivo:<ref> texto' é uma claim. Use get_decision com a \
referência D:<ref> para o motivo completo; se o pedido contrariar uma decisão, avise o usuário.";

const NOTHING_FOR_FILE: &str = "Nenhuma decisão ou regra ligada a esse arquivo no mapa do projeto.";

/// Why the backend could not answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendError {
    /// The xemnas app is closed or its local API is unreachable.
    AppClosed,
    /// The working directory is not a tracked project.
    ProjectNotFound,
    /// No decision matches the reference.
    NotFound,
    /// The reference matches more than one decision.
    Ambiguous,
    /// The reference or query is malformed.
    Invalid,
    /// Any other failure.
    Failed,
}

impl BackendError {
    /// Message shown to the agent.
    pub fn message(&self) -> &'static str {
        match self {
            Self::AppClosed => "O app xemnas está fechado; abra-o para consultar as decisões.",
            Self::ProjectNotFound => "Este diretório não é um projeto acompanhado pelo xemnas.",
            Self::NotFound => "Nenhuma decisão com essa referência neste projeto.",
            Self::Ambiguous => "Referência ambígua; use a referência completa, como D:bbbbcccc.",
            Self::Invalid => "Referência ou consulta inválida.",
            Self::Failed => "O xemnas não conseguiu responder agora.",
        }
    }
}

/// What the tools need from the running app.
pub trait Backend {
    /// Full decision text for a short reference. `project` is the repository
    /// the agent works in; `None` is the server's own directory.
    fn decision(&self, reference: &str, project: Option<&str>) -> Result<String, BackendError>;

    /// Compact block for a query (and a file it touches), or `None` when
    /// nothing is relevant.
    fn search(
        &self,
        query: &str,
        path: Option<&str>,
        project: Option<&str>,
    ) -> Result<Option<String>, BackendError>;

    /// Compact block of what holds for one file, or `None` when nothing does.
    fn file(&self, path: &str, project: Option<&str>) -> Result<Option<String>, BackendError>;
}

/// Handles one JSON-RPC message; notifications return `None`.
pub fn handle(message: &Value, backend: &dyn Backend) -> Option<Value> {
    let id = message.get("id").cloned()?;
    let method = message.get("method").and_then(Value::as_str).unwrap_or("");
    let params = message.get("params").cloned().unwrap_or(Value::Null);
    let outcome = match method {
        "initialize" => Ok(initialize(&params)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "tools/call" => call_tool(&params, backend),
        _ => Err((-32601, "método não suportado")),
    };
    Some(match outcome {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, text)) => error(id, code, text),
    })
}

/// Response for a line that is not valid JSON.
pub fn parse_error() -> Value {
    error(Value::Null, -32700, "JSON inválido")
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn initialize(params: &Value) -> Value {
    let version = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_PROTOCOL_VERSION);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": "xemnas", "version": env!("CARGO_PKG_VERSION") },
        "instructions": INSTRUCTIONS,
    })
}

fn tools() -> Value {
    json!([
        {
            "name": "get_decision",
            "description": "Abre uma decisão do projeto pela referência D:<ref> dos blocos \
                <xemnas-context>: motivo completo, premissas, relações e regras derivadas. \
                Parâmetro reference (alias aceito: ref). \
                Fora da pasta do projeto, passe project (caminho absoluto do repo).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "reference": { "type": "string", "description": "Ex.: D:bbbbcccc" },
                    "ref": { "type": "string", "description": "Alias de reference" },
                    "project": { "type": "string", "description": "Raiz do repo (absoluta)" }
                },
                "required": ["reference"],
                "additionalProperties": false
            }
        },
        {
            "name": "search_context",
            "description": "Busca decisões vigentes e regras do projeto sobre um assunto. \
                Resposta curta, uma linha por item, com referências para get_decision.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Assunto ou tarefa" },
                    "path": { "type": "string", "description": "Arquivo envolvido (opcional)" },
                    "project": { "type": "string", "description": "Raiz do repo (absoluta)" }
                },
                "required": ["query"],
                "additionalProperties": false
            }
        },
        {
            "name": "file_context",
            "description": "Decisões e regras que valem para um arquivo, pelo mapa do projeto. \
                Use antes de editar um arquivo.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Caminho absoluto, ou relativo ao project" },
                    "project": { "type": "string", "description": "Raiz do repo (absoluta)" }
                },
                "required": ["path"],
                "additionalProperties": false
            }
        }
    ])
}

/// Whether the repository of `project` is the one of `path` (relative to `project`
/// when it is not absolute), aliases and worktrees of one repository included.
/// Unknown places (nothing on disk, no repository) cannot disagree.
fn project_matches_path(project: &str, path: Option<&str>) -> bool {
    let Some(of_project) = locate_directory(project) else {
        return true;
    };
    let Some(of_path) = path.and_then(|path| resolve(path, project)) else {
        return true;
    };
    Commons::default().same(&of_project.top, &of_path.top)
}

/// `path` relative to the repository root `project` when it lies there by the disk
/// but not by its text (an alias, a `subst` drive, a sibling worktree).
fn relative_in_repository(project: &str, path: &str) -> Option<String> {
    let of_project = locate_directory(project)?;
    if !of_project.relative.is_empty() {
        return None;
    }
    let of_path = resolve(path, project)?;
    let same = Commons::default().same(&of_project.top, &of_path.top);
    (same && !of_path.relative.is_empty()).then_some(of_path.relative)
}

fn call_tool(params: &Value, backend: &dyn Backend) -> Result<Value, (i64, &'static str)> {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);
    let argument = |key: &str| {
        arguments
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    // The server is shared by agents working in different repositories: the project
    // is the `project` argument, else the repository of an absolute `path`, else
    // the server's own directory.
    let path_argument = argument("path");
    let project = match argument("project") {
        Some(project) => {
            if absolute(&project).is_none() {
                return Err((-32602, "project deve ser um caminho absoluto"));
            }
            if !project_matches_path(&project, path_argument.as_deref()) {
                return Err((-32602, "project não é o repositório do path"));
            }
            Some(project)
        }
        None => path_argument.as_deref().and_then(toplevel_of_path),
    };
    let project = project.as_deref();
    let in_project = |path: &str| match project {
        Some(root) => relative_inside(path, root)
            .or_else(|| relative_in_repository(root, path))
            .unwrap_or_else(|| path.to_string()),
        None => path.to_string(),
    };
    let outcome = match name {
        "get_decision" => {
            let reference = argument("reference")
                .or_else(|| argument("ref"))
                .ok_or((-32602, "informe reference"))?;
            backend.decision(&reference, project)
        }
        "search_context" => {
            let query = argument("query").ok_or((-32602, "informe query"))?;
            let path = path_argument.as_deref().map(in_project);
            backend
                .search(&query, path.as_deref(), project)
                .map(|found| {
                    found.unwrap_or_else(|| "Nada registrado sobre isso neste projeto.".to_string())
                })
        }
        "file_context" => {
            let path = path_argument.as_deref().ok_or((-32602, "informe path"))?;
            backend
                .file(&in_project(path), project)
                .map(|found| found.unwrap_or_else(|| NOTHING_FOR_FILE.to_string()))
        }
        _ => return Err((-32602, "ferramenta desconhecida")),
    };
    Ok(match outcome {
        Ok(text) => json!({ "content": [{ "type": "text", "text": text }], "isError": false }),
        Err(error) => json!({
            "content": [{ "type": "text", "text": error.message() }],
            "isError": true
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake;

    impl Backend for Fake {
        fn decision(&self, reference: &str, project: Option<&str>) -> Result<String, BackendError> {
            match reference {
                "D:bbbbcccc" => Ok("pergunta: Qual banco?".to_string()),
                "D:closed00" => Err(BackendError::AppClosed),
                "D:where000" => Ok(format!("project={project:?}")),
                _ => Err(BackendError::NotFound),
            }
        }

        fn file(&self, path: &str, project: Option<&str>) -> Result<Option<String>, BackendError> {
            Ok(match path {
                "src/db.rs" => Some("D:bbbbcccc Qual banco? → SQLite".to_string()),
                "src/where.rs" => Some(format!("project={project:?} path={path}")),
                _ => None,
            })
        }

        fn search(
            &self,
            query: &str,
            path: Option<&str>,
            project: Option<&str>,
        ) -> Result<Option<String>, BackendError> {
            Ok(match query {
                "banco" => Some("D:bbbbcccc v1 Qual banco? → SQLite".to_string()),
                "onde" => Some(format!("project={project:?} path={path:?}")),
                _ => None,
            })
        }
    }

    fn request(method: &str, params: Value) -> Value {
        json!({ "jsonrpc": "2.0", "id": 7, "method": method, "params": params })
    }

    fn call(name: &str, arguments: Value) -> Value {
        handle(
            &request(
                "tools/call",
                json!({ "name": name, "arguments": arguments }),
            ),
            &Fake,
        )
        .expect("response")
    }

    #[test]
    fn initialize_echoes_the_version_and_declares_tools() {
        let response = handle(
            &request("initialize", json!({ "protocolVersion": "2025-03-26" })),
            &Fake,
        )
        .expect("response");
        assert_eq!(response["id"], json!(7));
        assert_eq!(response["result"]["protocolVersion"], json!("2025-03-26"));
        assert_eq!(response["result"]["serverInfo"]["name"], json!("xemnas"));
        assert!(response["result"]["capabilities"]["tools"].is_object());
        assert!(response["result"]["instructions"]
            .as_str()
            .is_some_and(|text| text.contains("D:<ref>")));
    }

    #[test]
    fn lists_exactly_three_small_tools() {
        let response = handle(&request("tools/list", Value::Null), &Fake).expect("response");
        let tools = response["result"]["tools"].as_array().expect("tools");
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect();
        assert_eq!(
            names,
            vec!["get_decision", "search_context", "file_context"]
        );
        let size = serde_json::to_string(&response["result"])
            .expect("json")
            .len();
        assert!(size < 1_500, "tool definitions stay small: {size} bytes");
    }

    #[test]
    fn file_context_answers_or_says_nothing_is_tied() {
        let found = call("file_context", json!({ "path": "src/db.rs" }));
        assert_eq!(found["result"]["isError"], json!(false));
        assert!(found["result"]["content"][0]["text"]
            .as_str()
            .is_some_and(|text| text.contains("D:bbbbcccc")));
        let none = call("file_context", json!({ "path": "src/ui.rs" }));
        assert!(none["result"]["content"][0]["text"]
            .as_str()
            .is_some_and(|text| text.contains("Nenhuma decisão")));
        assert_eq!(
            call("file_context", json!({}))["error"]["code"],
            json!(-32602)
        );
    }

    #[test]
    fn tool_calls_return_text_or_a_friendly_error() {
        let ok = call("get_decision", json!({ "reference": "D:bbbbcccc" }));
        assert_eq!(ok["result"]["isError"], json!(false));
        assert_eq!(
            ok["result"]["content"][0]["text"],
            json!("pergunta: Qual banco?")
        );

        let alias = call("get_decision", json!({ "ref": "D:bbbbcccc" }));
        assert_eq!(alias["result"]["isError"], json!(false));
        assert_eq!(
            alias["result"]["content"][0]["text"],
            json!("pergunta: Qual banco?")
        );
        let both = call(
            "get_decision",
            json!({ "reference": "D:closed00", "ref": "D:bbbbcccc" }),
        );
        assert_eq!(both["result"]["isError"], json!(true), "reference wins");

        let closed = call("get_decision", json!({ "reference": "D:closed00" }));
        assert_eq!(closed["result"]["isError"], json!(true));
        assert!(closed["result"]["content"][0]["text"]
            .as_str()
            .is_some_and(|text| text.contains("fechado")));

        let empty = call("search_context", json!({ "query": "tela" }));
        assert_eq!(empty["result"]["isError"], json!(false));
        assert!(empty["result"]["content"][0]["text"]
            .as_str()
            .is_some_and(|text| text.starts_with("Nada")));
    }

    #[test]
    fn protocol_errors_use_json_rpc_codes() {
        assert_eq!(
            call("get_decision", json!({}))["error"]["code"],
            json!(-32602)
        );
        assert_eq!(
            call("delete_everything", json!({}))["error"]["code"],
            json!(-32602)
        );
        let unknown = handle(&request("resources/list", Value::Null), &Fake).expect("response");
        assert_eq!(unknown["error"]["code"], json!(-32601));
        assert_eq!(parse_error()["error"]["code"], json!(-32700));
        let notification = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        assert_eq!(handle(&notification, &Fake), None);
        let ping = handle(&request("ping", Value::Null), &Fake).expect("response");
        assert_eq!(ping["result"], json!({}));
    }
    fn text(response: &Value) -> String {
        response["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn the_project_is_the_argument_then_the_repo_of_an_absolute_path_then_the_servers_own() {
        // 1. Explicit `project`: relative paths stay relative, absolute ones are made relative to it.
        let explicit = call(
            "file_context",
            json!({ "project": "C:/repo", "path": "src/where.rs" }),
        );
        assert_eq!(
            text(&explicit),
            "project=Some(\"C:/repo\") path=src/where.rs"
        );
        let absolute = call(
            "file_context",
            json!({ "project": "C:/repo", "path": r"C:\repo\src\where.rs" }),
        );
        assert_eq!(
            text(&absolute),
            "project=Some(\"C:/repo\") path=src/where.rs"
        );
        let decision = call(
            "get_decision",
            json!({ "reference": "D:where000", "project": "C:/repo" }),
        );
        assert_eq!(text(&decision), "project=Some(\"C:/repo\")");

        // 2. No `project`, absolute `path`: the git work tree of the path (this very repository).
        let here = env!("CARGO_MANIFEST_DIR").replace('\\', "/");
        if let Some(top) = toplevel_of_path(&format!("{here}/src/protocol.rs")) {
            let found = text(&call(
                "search_context",
                json!({ "query": "onde", "path": format!("{here}/src/protocol.rs") }),
            ));
            assert_eq!(
                found,
                format!(
                    "project=Some({top:?}) path=Some({:?})",
                    relative_inside(&format!("{here}/src/protocol.rs"), &top).unwrap()
                )
            );
        }

        // 3. Neither: the server's own directory.
        let own = call("file_context", json!({ "path": "src/where.rs" }));
        assert_eq!(text(&own), "project=None path=src/where.rs");
        let own_search = call("search_context", json!({ "query": "onde" }));
        assert_eq!(text(&own_search), "project=None path=None");
    }

    #[test]
    fn a_project_that_is_not_an_absolute_path_is_refused() {
        for project in ["", "repo", "./repo", "C:repo"] {
            let response = call(
                "file_context",
                json!({ "path": "src/where.rs", "project": project }),
            );
            assert_eq!(response["error"]["code"], json!(-32602), "{project:?}");
        }
    }

    #[test]
    fn a_project_of_another_repository_than_the_absolute_path_is_refused() {
        let here = env!("CARGO_MANIFEST_DIR").replace('\\', "/");
        let file = format!("{here}/src/protocol.rs");
        let Some(top) = toplevel_of_path(&file) else {
            return;
        };
        let other = std::env::temp_dir().join(format!("xemnas-proto-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&other);
        std::fs::create_dir_all(&other).expect("dir");
        let started = std::process::Command::new("git")
            .arg("-C")
            .arg(&other)
            .args(["init", "-q"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
        if started {
            let refused = call(
                "file_context",
                json!({ "path": file, "project": other.to_string_lossy() }),
            );
            assert_eq!(refused["error"]["code"], json!(-32602));
        }
        // The repository's own root, and a subfolder of it, agree with the path.
        for project in [top.clone(), here.clone()] {
            let ok = call("file_context", json!({ "path": file, "project": project }));
            assert!(ok.get("error").is_none(), "{ok}");
        }
        let _ = std::fs::remove_dir_all(&other);
    }

    #[test]
    fn a_relative_path_into_another_repository_than_the_project_is_refused() {
        let base = std::env::temp_dir().join(format!("xemnas-proto-rel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let project = base.join("a");
        let nested = project.join("nested");
        std::fs::create_dir_all(&nested).expect("dir");
        let started = [&project, &nested].iter().all(|folder| {
            std::process::Command::new("git")
                .arg("-C")
                .arg(folder)
                .args(["init", "-q"])
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
        });
        if started {
            let project = project.to_string_lossy().into_owned();
            let refused = call(
                "file_context",
                json!({ "path": "nested/two.rs", "project": project }),
            );
            assert_eq!(refused["error"]["code"], json!(-32602));
            let ok = call(
                "file_context",
                json!({ "path": "src/one.rs", "project": project }),
            );
            assert!(ok.get("error").is_none(), "{ok}");
        }
        let _ = std::fs::remove_dir_all(&base);
    }
}
