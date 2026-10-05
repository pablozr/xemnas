//! The Claude Code CLI installed on this machine as an extraction provider
//! (ADR-0004).
//!
//! xemnas never signs in to Anthropic: it runs the `claude` binary the user
//! already signed in with `claude auth login`, and the CLI uses its own
//! login. Each call is one print-mode turn with no tools, no MCP servers, no
//! settings, hooks or CLAUDE.md, in an empty working directory, so the plan
//! pays only for xemnas' own prompt and the call cannot touch the project.

use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use application::extract::{
    CandidateExtractor, CandidateProposal, DecisionEvidence, ExtractError, ExtractionBackground,
    RelevanceSignal,
};
use application::profile::{consent_status, AiProfile, ProfileKind};
use application::providers::{ClaudeCodeProbe, ClaudeCodeStatus, ModelInfo, ProviderError};

use crate::opencode::json_object;
use crate::{
    build_user_content, output_schema, parse_model_output, Attempt, RetryPolicy,
    MAX_RESPONSE_BYTES, SYSTEM_PROMPT,
};

/// Environment variable that points xemnas at a specific `claude` binary.
pub const CLAUDE_CODE_BIN_ENV: &str = "XEMNAS_CLAUDE_CODE";

/// One call: process start (~2 s measured), the turn and the structured
/// output round.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(180);
/// `claude auth status` and `--version` answer locally.
const PROBE_TIMEOUT: Duration = Duration::from_secs(15);
/// How often a running call is checked for exit or timeout.
const POLL: Duration = Duration::from_millis(50);
/// Cap on extended thinking, which Claude Code turns on by default and
/// `--effort` does not reach on Haiku. One synthetic extraction with Haiku
/// (2026-10-05, one run each): default 6.6k-11k output tokens and 72-122 s;
/// 1024 → 1.6k tokens and 21 s; none → ~0.5k tokens and 9 s, but it found
/// the rule in only one of three runs.
const THINKING_BUDGET: &str = "1024";
/// Pause after the plan's usage limit: retrying sooner only fails again.
const USAGE_LIMIT_PAUSE: Duration = Duration::from_secs(15 * 60);

/// The model aliases Claude Code resolves to the latest of each family.
pub const CLAUDE_CODE_MODELS: &[(&str, &str)] = &[
    ("haiku", "Claude Haiku"),
    ("sonnet", "Claude Sonnet"),
    ("opus", "Claude Opus"),
];

/// The models offered for Claude Code; fixed, so listing costs no call.
pub fn claude_code_models() -> Vec<ModelInfo> {
    CLAUDE_CODE_MODELS
        .iter()
        .map(|(id, label)| ModelInfo {
            id: (*id).to_string(),
            label: (*label).to_string(),
        })
        .collect()
}

/// The `claude` binary: [`CLAUDE_CODE_BIN_ENV`], the `PATH` (resolving the
/// npm shim on Windows to the native executable it wraps) or the native
/// installer's folder.
pub fn find_claude_code() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os(CLAUDE_CODE_BIN_ENV).map(PathBuf::from) {
        return path.is_file().then_some(path);
    }
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" });
    locate(std::env::var_os("PATH"), home.map(PathBuf::from))
}

fn locate(path: Option<OsString>, home: Option<PathBuf>) -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        "claude.exe"
    } else {
        "claude"
    };
    for dir in path.iter().flat_map(std::env::split_paths) {
        let direct = dir.join(exe);
        if direct.is_file() {
            return Some(direct);
        }
        // `npm i -g` puts a `claude.cmd` shim on the PATH; batch files are
        // not run directly, so go to the executable it calls.
        if cfg!(windows) && dir.join("claude.cmd").is_file() {
            let native = dir.join("node_modules/@anthropic-ai/claude-code/bin/claude.exe");
            if native.is_file() {
                return Some(native);
            }
        }
    }
    let home = home?;
    [
        home.join(".local/bin").join(exe),
        home.join(".claude/local").join(exe),
    ]
    .into_iter()
    .find(|candidate| candidate.is_file())
}

/// Output of one finished CLI run.
#[derive(Debug, Clone, Default)]
pub struct RunOutput {
    /// Standard output, cut at [`MAX_RESPONSE_BYTES`] plus one byte.
    pub stdout: Vec<u8>,
    /// Whether the process was killed at the timeout.
    pub timed_out: bool,
}

/// Runs the CLI: `(arguments, standard input, timeout)`. Replaced in tests.
pub type Runner =
    Arc<dyn Fn(&[OsString], &str, Duration) -> std::io::Result<RunOutput> + Send + Sync>;

/// Runs `binary` in `workdir` without a console window, feeding `stdin`
/// and killing it at `timeout`. Standard error is dropped: it may echo
/// local paths and never reaches the user.
fn run_process(
    binary: &Path,
    workdir: &Path,
    args: &[OsString],
    stdin: &str,
    timeout: Duration,
) -> std::io::Result<RunOutput> {
    let mut command = Command::new(binary);
    command
        .args(args)
        .current_dir(workdir)
        .env("MAX_THINKING_TOKENS", THINKING_BUDGET)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn()?;
    let input = stdin.to_owned();
    let mut pipe_in = child.stdin.take();
    let writer = std::thread::spawn(move || {
        if let Some(pipe) = pipe_in.as_mut() {
            let _ = pipe.write_all(input.as_bytes());
        }
        drop(pipe_in);
    });
    let pipe_out = child.stdout.take();
    let reader = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        if let Some(pipe) = pipe_out {
            let _ = pipe
                .take(MAX_RESPONSE_BYTES as u64 + 1)
                .read_to_end(&mut buffer);
        }
        buffer
    });
    let deadline = Instant::now() + timeout;
    let mut timed_out = false;
    while child.try_wait()?.is_none() {
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            timed_out = true;
            break;
        }
        std::thread::sleep(POLL);
    }
    let _ = writer.join();
    let stdout = reader.join().unwrap_or_default();
    Ok(RunOutput { stdout, timed_out })
}

/// Folder the CLI runs in: empty and owned by xemnas, so no project
/// CLAUDE.md, settings or hooks apply.
fn workdir() -> std::io::Result<PathBuf> {
    let dir = std::env::temp_dir().join("xemnas-claude-code");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Print-mode arguments for one structured answer.
pub(crate) fn call_args(model: &str, system: &str, schema: &serde_json::Value) -> Vec<OsString> {
    [
        "-p",
        "--output-format",
        "json",
        "--model",
        model,
        "--system-prompt",
        system,
        "--json-schema",
        &schema.to_string(),
        // Meant to spend less on Sonnet and Opus (not measured); Haiku
        // ignores it, hence [`THINKING_BUDGET`].
        "--effort",
        "low",
        // No tools: the context is already in the prompt.
        "--tools",
        "",
        // Not even the user's MCP servers (xemnas' own included).
        "--strict-mcp-config",
        // No user/project settings, so none of their hooks run either.
        "--setting-sources",
        "",
        "--no-session-persistence",
    ]
    .into_iter()
    .map(OsString::from)
    .collect()
}

/// Classifies the `--output-format json` result of one call.
pub(crate) fn read_result(output: &RunOutput) -> Attempt<String> {
    if output.timed_out {
        return Attempt::Transient;
    }
    if output.stdout.len() > MAX_RESPONSE_BYTES {
        return Attempt::Fatal(ExtractError::Extractor(
            "a resposta do Claude Code excedeu o limite".to_string(),
        ));
    }
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return Attempt::Fatal(ExtractError::Extractor(
            "o Claude Code respondeu num formato inesperado".to_string(),
        ));
    };
    let text = value
        .get("result")
        .and_then(|result| result.as_str())
        .unwrap_or_default();
    if value.get("is_error").and_then(|flag| flag.as_bool()) == Some(true) {
        return classify_error(
            value
                .get("api_error_status")
                .and_then(|status| status.as_u64()),
            text,
        );
    }
    if let Some(structured) = value.get("structured_output").filter(|v| v.is_object()) {
        return Attempt::Success(structured.to_string());
    }
    if text.trim().is_empty() {
        return Attempt::Fatal(ExtractError::Extractor(
            "o Claude Code não devolveu texto".to_string(),
        ));
    }
    Attempt::Success(json_object(text).to_string())
}

fn classify_error(status: Option<u64>, text: &str) -> Attempt<String> {
    let lower = text.to_lowercase();
    let says = |needles: &[&str]| needles.iter().any(|needle| lower.contains(needle));
    if says(&["not logged in", "/login", "invalid api key", "oauth token"])
        || matches!(status, Some(401 | 403))
    {
        return Attempt::Fatal(ExtractError::Extractor(
            "o Claude Code não está conectado: rode `claude auth login`".to_string(),
        ));
    }
    if says(&["usage limit", "hit your limit", "limit reached"]) {
        return Attempt::RateLimited(Some(USAGE_LIMIT_PAUSE));
    }
    if status == Some(429) || says(&["rate limit"]) {
        return Attempt::RateLimited(None);
    }
    if status.is_some_and(|code| code >= 500) || says(&["overloaded"]) {
        return Attempt::Transient;
    }
    Attempt::Fatal(ExtractError::Extractor(
        "o Claude Code recusou a chamada".to_string(),
    ))
}

/// Extractor through the local Claude Code.
pub struct ClaudeCodeExtractor {
    profile: AiProfile,
    runner: Runner,
    retry: RetryPolicy,
    sleep: Arc<dyn Fn(Duration) + Send + Sync>,
}

impl std::fmt::Debug for ClaudeCodeExtractor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ClaudeCodeExtractor")
            .field("model", &self.profile.model)
            .finish_non_exhaustive()
    }
}

impl ClaudeCodeExtractor {
    /// Builds the extractor for a validated profile over the installed CLI.
    pub fn new(profile: &AiProfile) -> Result<Self, ExtractError> {
        let binary = find_claude_code().ok_or_else(|| {
            ExtractError::Extractor("o Claude Code não foi encontrado nesta máquina".to_string())
        })?;
        let dir = workdir().map_err(|_| {
            ExtractError::Extractor("não foi possível preparar a pasta do Claude Code".to_string())
        })?;
        let runner: Runner =
            Arc::new(move |args, stdin, timeout| run_process(&binary, &dir, args, stdin, timeout));
        Self::with_runner(profile, runner)
    }

    /// Builds the extractor over a given runner (tests use a fake CLI).
    pub fn with_runner(profile: &AiProfile, runner: Runner) -> Result<Self, ExtractError> {
        if profile.kind != ProfileKind::ClaudeCode {
            return Err(ExtractError::Extractor(
                "o perfil não usa o Claude Code".to_string(),
            ));
        }
        profile
            .validate()
            .map_err(|_| ExtractError::Extractor("perfil de IA inválido".to_string()))?;
        Ok(Self {
            profile: profile.clone(),
            runner,
            retry: RetryPolicy::default(),
            sleep: Arc::new(|delay: Duration| std::thread::sleep(delay)),
        })
    }

    /// Replaces the sleep function; tests inject a no-op to stay fast.
    pub fn with_sleep(mut self, sleep: Arc<dyn Fn(Duration) + Send + Sync>) -> Self {
        self.sleep = sleep;
        self
    }

    pub(crate) fn profile(&self) -> &AiProfile {
        &self.profile
    }

    /// Asks Claude Code for one JSON answer, with consent and retries.
    pub fn complete(
        &self,
        system: &str,
        user: &str,
        schema_name: &str,
        schema: &serde_json::Value,
    ) -> Result<String, ExtractError> {
        self.complete_checked(system, user, schema_name, schema, None)
    }

    pub(crate) fn complete_checked(
        &self,
        system: &str,
        user: &str,
        _schema_name: &str,
        schema: &serde_json::Value,
        authorization: Option<&dyn application::external::Authorization>,
    ) -> Result<String, ExtractError> {
        if let Err(reason) = consent_status(&self.profile) {
            return Err(ExtractError::Extractor(format!(
                "chamadas externas bloqueadas: {reason}"
            )));
        }
        let args = call_args(&self.profile.model, system, schema);
        let mut attempt = 1u32;
        loop {
            if let Some(authorization) = authorization {
                authorization.check()?;
            }
            let outcome = match (self.runner)(&args, user, REQUEST_TIMEOUT) {
                Ok(output) => read_result(&output),
                Err(_) => Attempt::Fatal(ExtractError::Extractor(
                    "não foi possível executar o Claude Code".to_string(),
                )),
            };
            match outcome {
                Attempt::Success(text) => return Ok(text),
                Attempt::Fatal(error) => return Err(error),
                Attempt::RateLimited(retry_after) => {
                    return Err(ExtractError::RateLimited { retry_after })
                }
                Attempt::Transient => {
                    if attempt >= self.retry.max_attempts {
                        return Err(ExtractError::Unavailable { attempts: attempt });
                    }
                    (self.sleep)(self.retry.delay_for(attempt));
                    attempt += 1;
                }
            }
        }
    }
}

impl CandidateExtractor for ClaudeCodeExtractor {
    fn extract(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        self.extract_with(input, signals, &ExtractionBackground::default())
    }

    fn extract_with(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
        background: &ExtractionBackground,
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        let user = build_user_content(&self.profile, input, signals, background);
        let text = self.complete(
            SYSTEM_PROMPT,
            &user,
            "decision_candidates",
            &output_schema(),
        )?;
        parse_model_output(&text, signals)
    }
}

/// Reads `claude auth status --json`; spends no tokens.
#[derive(Debug, Clone, Copy, Default)]
pub struct LocalClaudeCode;

impl ClaudeCodeProbe for LocalClaudeCode {
    fn status(&self) -> Result<ClaudeCodeStatus, ProviderError> {
        let binary = find_claude_code().ok_or(ProviderError::NotInstalled)?;
        let dir = workdir().map_err(|_| ProviderError::Unreachable)?;
        let run = |args: &[&str]| {
            let args: Vec<OsString> = args.iter().map(OsString::from).collect();
            run_process(&binary, &dir, &args, "", PROBE_TIMEOUT)
                .map_err(|_| ProviderError::NotInstalled)
        };
        let version = run(&["--version"])?.stdout;
        let auth = run(&["auth", "status", "--json"])?;
        let mut status = parse_auth_status(&auth.stdout).ok_or(ProviderError::InvalidResponse)?;
        status.version = parse_version(&version);
        Ok(status)
    }
}

/// `2.1.289 (Claude Code)` → `2.1.289`.
fn parse_version(stdout: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(stdout);
    let version = text.split_whitespace().next()?;
    version
        .chars()
        .all(|c| c.is_ascii_digit() || c == '.')
        .then(|| version.to_string())
}

/// The fields of `claude auth status --json` the settings screen shows.
pub(crate) fn parse_auth_status(stdout: &[u8]) -> Option<ClaudeCodeStatus> {
    let value = serde_json::from_slice::<serde_json::Value>(stdout).ok()?;
    let text = |key: &str| {
        value
            .get(key)
            .and_then(|field| field.as_str())
            .filter(|field| !field.is_empty() && *field != "none")
            .map(str::to_string)
    };
    Some(ClaudeCodeStatus {
        version: None,
        signed_in: value.get("loggedIn")?.as_bool()?,
        auth_method: text("authMethod"),
        email: text("email"),
        subscription: text("subscriptionType"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(json: &str) -> RunOutput {
        RunOutput {
            stdout: json.as_bytes().to_vec(),
            timed_out: false,
        }
    }

    fn fatal_message(attempt: Attempt<String>) -> String {
        match attempt {
            Attempt::Fatal(ExtractError::Extractor(message)) => message,
            _ => panic!("expected a fatal error"),
        }
    }

    #[test]
    fn call_runs_without_tools_settings_or_mcp() {
        let args: Vec<String> = call_args("haiku", "sys", &serde_json::json!({"type": "object"}))
            .into_iter()
            .map(|arg| arg.into_string().expect("utf-8"))
            .collect();
        let after = |flag: &str| {
            let at = args.iter().position(|arg| arg == flag).expect(flag);
            args[at + 1].clone()
        };
        assert_eq!(args[0], "-p");
        assert_eq!(after("--output-format"), "json");
        assert_eq!(after("--model"), "haiku");
        assert_eq!(after("--system-prompt"), "sys");
        assert_eq!(after("--json-schema"), r#"{"type":"object"}"#);
        assert_eq!(after("--effort"), "low");
        assert_eq!(after("--tools"), "");
        assert_eq!(after("--setting-sources"), "");
        assert!(args.iter().any(|arg| arg == "--strict-mcp-config"));
        assert!(args.iter().any(|arg| arg == "--no-session-persistence"));
    }

    #[test]
    fn structured_output_wins_over_the_text_result() {
        let attempt = read_result(&output(
            r#"{"type":"result","is_error":false,"result":"{\"answer\":\"Hi\"}","structured_output":{"answer":"Hey"}}"#,
        ));
        assert!(matches!(attempt, Attempt::Success(text) if text == r#"{"answer":"Hey"}"#));
        let fenced = read_result(&output(
            r#"{"type":"result","is_error":false,"result":"```json\n{\"proposals\":[]}\n```"}"#,
        ));
        assert!(matches!(fenced, Attempt::Success(text) if text == r#"{"proposals":[]}"#));
    }

    #[test]
    fn logged_out_cli_asks_for_claude_auth_login() {
        // Shape captured from Claude Code 2.1.289 with an empty config dir.
        let attempt = read_result(&output(
            r#"{"type":"result","subtype":"success","is_error":true,"api_error_status":null,"result":"Not logged in · Please run /login"}"#,
        ));
        assert!(fatal_message(attempt).contains("claude auth login"));
    }

    #[test]
    fn limits_pause_and_server_errors_retry() {
        let usage = read_result(&output(
            r#"{"is_error":true,"result":"You've hit your limit · resets 3pm"}"#,
        ));
        assert!(matches!(usage, Attempt::RateLimited(Some(pause)) if pause == USAGE_LIMIT_PAUSE));
        let rate = read_result(&output(
            r#"{"is_error":true,"api_error_status":429,"result":"API Error"}"#,
        ));
        assert!(matches!(rate, Attempt::RateLimited(None)));
        let overloaded = read_result(&output(
            r#"{"is_error":true,"api_error_status":529,"result":"Overloaded"}"#,
        ));
        assert!(matches!(overloaded, Attempt::Transient));
        let timeout = read_result(&RunOutput {
            stdout: Vec::new(),
            timed_out: true,
        });
        assert!(matches!(timeout, Attempt::Transient));
        assert!(matches!(
            read_result(&output("not json")),
            Attempt::Fatal(_)
        ));
    }

    #[test]
    fn auth_status_reports_plan_without_credentials() {
        let status = parse_auth_status(
            br#"{"loggedIn":true,"authMethod":"claude.ai","email":"a@b.c","subscriptionType":"max","orgId":"x"}"#,
        )
        .expect("status");
        assert!(status.signed_in);
        assert_eq!(status.auth_method.as_deref(), Some("claude.ai"));
        assert_eq!(status.email.as_deref(), Some("a@b.c"));
        assert_eq!(status.subscription.as_deref(), Some("max"));
        let out = parse_auth_status(br#"{"loggedIn":false,"authMethod":"none"}"#).expect("status");
        assert!(!out.signed_in);
        assert_eq!(out.auth_method, None);
        assert_eq!(
            parse_version(b"2.1.289 (Claude Code)\n").as_deref(),
            Some("2.1.289")
        );
    }

    fn consented_profile() -> AiProfile {
        use application::profile::{build_preview, grant_consent, offline_default_profile};
        let mut profile = offline_default_profile();
        profile.kind = ProfileKind::ClaudeCode;
        profile.model = "haiku".to_string();
        grant_consent(
            &profile,
            &build_preview(&profile),
            "2026-10-05T00:00:00Z",
            true,
        )
        .expect("consent")
    }

    #[test]
    fn retries_a_timeout_then_returns_the_answer_from_stdin_prompt() {
        use std::sync::Mutex;
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let seen = calls.clone();
        let runner: Runner = Arc::new(move |_args, stdin, _timeout| {
            let mut seen = seen.lock().expect("lock");
            seen.push(stdin.to_string());
            Ok(if seen.len() == 1 {
                RunOutput {
                    stdout: Vec::new(),
                    timed_out: true,
                }
            } else {
                output(r#"{"is_error":false,"structured_output":{"ok":true}}"#)
            })
        });
        let extractor = ClaudeCodeExtractor::with_runner(&consented_profile(), runner)
            .expect("extractor")
            .with_sleep(Arc::new(|_| {}));
        let answer = extractor
            .complete(
                "sys",
                "the turn",
                "x",
                &serde_json::json!({"type": "object"}),
            )
            .expect("answer");
        assert_eq!(answer, r#"{"ok":true}"#);
        assert_eq!(*calls.lock().expect("lock"), vec!["the turn", "the turn"]);
    }

    #[test]
    fn never_runs_without_consent() {
        let mut profile = consented_profile();
        profile.external_calls_enabled = false;
        let runner: Runner = Arc::new(|_, _, _| panic!("must not run"));
        let extractor = ClaudeCodeExtractor::with_runner(&profile, runner).expect("extractor");
        assert!(extractor
            .complete("sys", "user", "x", &serde_json::json!({}))
            .is_err());
    }

    #[test]
    #[ignore = "runs the signed-in Claude Code and spends plan usage; run explicitly as evidence"]
    fn live_claude_code_answers_the_extraction_schema() {
        let binary = find_claude_code().expect("claude installed");
        let dir = workdir().expect("workdir");
        // Prints what each run cost, as evidence for the ADR's numbers.
        let runner: Runner = Arc::new(move |args, stdin, timeout| {
            let started = Instant::now();
            let output = run_process(&binary, &dir, args, stdin, timeout)?;
            let value: serde_json::Value =
                serde_json::from_slice(&output.stdout).unwrap_or_default();
            eprintln!(
                "wall_ms={} api_ms={} turns={} usage={} timed_out={}",
                started.elapsed().as_millis(),
                value["duration_api_ms"],
                value["num_turns"],
                value["usage"],
                output.timed_out
            );
            Ok(output)
        });
        let extractor = ClaudeCodeExtractor::with_runner(&consented_profile(), runner)
            .expect("extractor")
            .with_sleep(Arc::new(|_| {}));
        let user = "### artifact a1 user_text\nVamos guardar tudo em SQLite com migrações \
                    só para frente; nada de apagar coluna.\n### artifact a2 assistant_text\n\
                    Fechado: toda migração é forward-only, registrado no ADR.";
        let answer = extractor
            .complete(SYSTEM_PROMPT, user, "decision_candidates", &output_schema())
            .expect("answer");
        eprintln!("answer={answer}");
        let value: serde_json::Value = serde_json::from_str(&answer).expect("json");
        assert!(value.get("proposals").is_some_and(|p| p.is_array()));
    }

    #[test]
    fn finds_the_native_exe_behind_the_npm_shim() {
        let root =
            std::env::temp_dir().join(format!("xemnas-claude-locate-{}", std::process::id()));
        let npm = root.join("npm");
        let home = root.join("home");
        let _ = std::fs::remove_dir_all(&root);
        let exe = if cfg!(windows) {
            "claude.exe"
        } else {
            "claude"
        };
        if cfg!(windows) {
            let native = npm.join("node_modules/@anthropic-ai/claude-code/bin");
            std::fs::create_dir_all(&native).expect("dirs");
            std::fs::write(npm.join("claude.cmd"), "").expect("shim");
            std::fs::write(native.join(exe), "").expect("exe");
            assert_eq!(
                locate(Some(npm.clone().into_os_string()), None),
                Some(native.join(exe))
            );
        }
        let local = home.join(".local/bin");
        std::fs::create_dir_all(&local).expect("dirs");
        std::fs::write(local.join(exe), "").expect("exe");
        assert_eq!(
            locate(
                Some(root.join("missing").into_os_string()),
                Some(home.clone())
            ),
            Some(local.join(exe))
        );
        assert_eq!(locate(None, None), None);
        let _ = std::fs::remove_dir_all(&root);
    }
}
