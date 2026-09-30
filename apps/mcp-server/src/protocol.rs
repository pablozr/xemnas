//! Minimal MCP (JSON-RPC 2.0 over stdio): `initialize`, `ping`, `tools/list`, `tools/call`.

use serde_json::{json, Value};

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
    /// Full decision text for a short reference.
    fn decision(&self, reference: &str) -> Result<String, BackendError>;

    /// Compact block for a query (and a file it touches), or `None` when
    /// nothing is relevant.
    fn search(&self, query: &str, path: Option<&str>) -> Result<Option<String>, BackendError>;

    /// Compact block of what holds for one file, or `None` when nothing does.
    fn file(&self, path: &str) -> Result<Option<String>, BackendError>;
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
                <xemnas-context>: motivo completo, premissas, relações e regras derivadas.",
            "inputSchema": {
                "type": "object",
                "properties": { "reference": { "type": "string", "description": "Ex.: D:bbbbcccc" } },
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
                    "path": { "type": "string", "description": "Arquivo envolvido (opcional)" }
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
                "properties": { "path": { "type": "string", "description": "Ex.: src/app.rs" } },
                "required": ["path"],
                "additionalProperties": false
            }
        }
    ])
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
    let outcome = match name {
        "get_decision" => {
            let reference = argument("reference").ok_or((-32602, "informe reference"))?;
            backend.decision(&reference)
        }
        "search_context" => {
            let query = argument("query").ok_or((-32602, "informe query"))?;
            let path = argument("path");
            backend.search(&query, path.as_deref()).map(|found| {
                found.unwrap_or_else(|| "Nada registrado sobre isso neste projeto.".to_string())
            })
        }
        "file_context" => {
            let path = argument("path").ok_or((-32602, "informe path"))?;
            backend
                .file(&path)
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
        fn decision(&self, reference: &str) -> Result<String, BackendError> {
            match reference {
                "D:bbbbcccc" => Ok("pergunta: Qual banco?".to_string()),
                "D:closed00" => Err(BackendError::AppClosed),
                _ => Err(BackendError::NotFound),
            }
        }

        fn file(&self, path: &str) -> Result<Option<String>, BackendError> {
            Ok((path == "src/db.rs").then(|| "D:bbbbcccc Qual banco? → SQLite".to_string()))
        }

        fn search(&self, query: &str, _path: Option<&str>) -> Result<Option<String>, BackendError> {
            Ok((query == "banco").then(|| "D:bbbbcccc v1 Qual banco? → SQLite".to_string()))
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
}
