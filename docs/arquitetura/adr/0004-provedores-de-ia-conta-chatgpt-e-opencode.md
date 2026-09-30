# ADR-0004: Provedores de IA — modelo local, conta ChatGPT e OpenCode

- **Status:** aceito
- **Data:** 2026-09-30
- **Contexto:** o MVP entregou dois extratores: a heurística local (`Fake`) e um provedor compatível com OpenAI por chave (`OpenAiCompatible`). O MVP-SPEC §12 pedia "apenas um provider real de menor esforço" e dizia que o gateway do OpenCode "é orquestrador, não modelo". Na prática, a chave paga afasta o uso real, e modelos locais sem chave não conseguem consentir. A pesquisa `docs/pesquisas/provedores-e-modelos-de-ia.md` levantou três caminhos sem chave paga. O usuário decidiu implementar os três, pular a semana de dogfood antes disso e seguir direto para backend e telas. Este ADR revisa o §12 nesses pontos e mantém todas as outras regras dele.

## Decisão

O perfil de IA passa a ter quatro tipos (`ProfileKind`). Os dois existentes não mudam de nome, então perfis salvos continuam carregando e os consentimentos já dados continuam válidos.

| Tipo | Para quem | Autenticação | Destino dos dados |
| --- | --- | --- | --- |
| `fake` | Heurística local, padrão | nenhuma | nada sai da máquina |
| `open_ai_compatible` | Chave de API **ou** modelo local (Ollama, LM Studio) | chave no cofre; **opcional** quando o endereço é loopback | o endereço configurado |
| `chat_gpt_plan` | Conta ChatGPT, usando o plano do usuário | Sign in with ChatGPT (OAuth com PKCE) | `https://api.openai.com/v1`, na conta do usuário |
| `open_code` | OpenCode Zen (por uso) ou OpenCode Go (assinatura) | chave de API do OpenCode, no cofre | `https://opencode.ai/zen/v1` ou `https://opencode.ai/zen/go/v1`, que repassam ao provedor do modelo |

### Regras comuns (continuam as do §12)

- Qualquer tipo diferente de `fake` só é usado depois do **consentimento** sobre a prévia do que sai da máquina. O hash da prévia inclui o tipo, o destino, o modelo e, na conta ChatGPT, a conta conectada. Trocar qualquer um deles pede novo consentimento.
- A saída do modelo continua com esquema estrito e a mesma validação de lote; texto do modelo nunca vira SQL, caminho ou comando; falha do provedor não bloqueia a captura nem a UI; nada confirma decisão sozinho.
- Segredos (chaves de API e refresh token) ficam só no cofre do sistema (Credential Manager no Windows). O domínio não conhece nomes de fornecedores.
- Toda chamada de rede de catálogo, login ou extração roda fora da thread de UI, com tempo limite.

### Catálogo de modelos

Um port `ModelCatalog` lista os modelos do destino configurado: `GET {endpoint}/models` para compatíveis com OpenAI (formato `data[].id`); `GET /v1/models` com o token do plano para a conta ChatGPT (formato `models[]` com `slug`, `display_name` e `visibility`, mostrando só `visibility: "list"`); e `GET {gateway}/models` no OpenCode Zen ou Go (público, formato `data[].id`), mostrando só as famílias que o xemnas sabe chamar. Falha vira lista vazia com motivo; digitar o modelo à mão continua possível.

### Conta ChatGPT (Sign in with ChatGPT para apps open source)

Segue a documentação oficial (`developers.openai.com/siwc/token-sharing-open-source`), conferida em 30/09/2026:

- **Login:** navegador na própria máquina, callback em `http://127.0.0.1:{porta}/auth/callback`, `response_type=code`, PKCE `S256`, `state` e `nonce` novos a cada tentativa. Endpoints do issuer `https://auth.openai.com`: autorização `/api/accounts/authorize`, token `/api/accounts/oauth/token`, revogação `/api/accounts/oauth/revoke`, userinfo `/api/accounts/oauth/userinfo`.
- **Registro:** o primeiro login usa `client_id=dynamic_agent_client`, `agent_name_hint=xemnas` e um `ext_agent_host_id` estável por instalação (`urn:uuid:…`). O callback devolve o `client_id` emitido (`oaiapp_…`), que é guardado; `dynamic_agent_client` nunca é guardado. Logins seguintes reutilizam o `client_id` emitido.
- **Escopos:** `openid profile email offline_access resource.invoke chatgpt.tokens.use.direct`, com `resource=https://api.openai.com/v1`. Sem `chatgpt.tokens.use.direct` concedido, o login fica válido, mas o uso do plano fica desligado e a tela diz isso.
- **Tokens:** o access token vale 1 h e fica só em memória; o refresh token (30 dias, rotaciona a cada uso) fica só no cofre. O perfil guarda apenas dados não secretos: `client_id` emitido, `ext_agent_host_id`, e-mail e `sub`.
- **ID token:** chega direto do endpoint de token, por TLS, na troca com PKCE. O xemnas valida `iss`, `aud` (o `client_id` emitido), `exp` e `nonce`, e confirma a identidade no `userinfo`, como o OpenID Connect Core §3.1.3.7 permite para tokens recebidos diretamente do endpoint de token. A verificação da assinatura RS256 pelo JWKS fica como pendência: exige uma biblioteca de RSA que não está no workspace, e crates novas com build script são bloqueadas pelo Controle de Aplicativo da máquina de desenvolvimento. Registrar quando for adicionada.
- **Inferência:** `POST https://api.openai.com/v1/responses` com `store: false` e `stream: true` (obrigatórios), `instructions` e `input`, saída estruturada por `text.format` com `json_schema` estrito. Campos proibidos no fluxo (por exemplo `temperature`, `max_output_tokens`, `metadata`) não são enviados. Só `response.completed` conta como sucesso; os erros `subscription_sharing_*` viram mensagens de produto: limite do plano atingido leva a "Manage usage" (`https://chatgpt.com/settings/usage`).
- **Sair:** revoga o refresh token no endpoint de revogação e apaga o token do cofre. Se a revogação falhar por rede, apaga localmente e avisa que dá para desconectar em ChatGPT › Configurações. O `client_id` emitido e o `ext_agent_host_id` ficam para um próximo login.
- **Interface:** botão "Continue with ChatGPT", aviso "Using ChatGPT plan" perto do modelo, link "Manage usage" e, no primeiro login, o aviso de que o uso consome o plano, conforme as diretrizes de UI da OpenAI.

### OpenCode Zen e OpenCode Go (revisa o §12)

Revisado em 30/09/2026. A primeira versão falava com o servidor local do OpenCode (`opencode serve`, sessão descartável por extração e filtro no plugin). Na prática, "OpenCode" em outros apps é o **gateway de modelos** do OpenCode, configurado só com a chave, e é isso que o usuário espera. O modo de servidor local saiu, junto com o filtro de sessões no plugin.

- O usuário cria a chave em `https://opencode.ai/auth`; a mesma chave serve para o Zen (`https://opencode.ai/zen/v1`, por uso, com modelos gratuitos) e o Go (`https://opencode.ai/zen/go/v1`, assinatura). O perfil só aceita esses dois endereços.
- O gateway usa um protocolo por família de modelo (tabelas de `opencode.ai/docs/zen` e `opencode.ai/docs/go`, conferidas em 30/09/2026): GPT, Grok e Muse em `/responses` (com `store: false` e `json_schema` estrito); Claude em `/messages` (Anthropic, `x-api-key` e `anthropic-version`); os demais em `/chat/completions` com `response_format: json_object`. Exceções: Qwen vai por `/messages` (menos Qwen3.8 Max no Zen) e MiniMax por `/messages` no Go. Gemini e System One (endpoints próprios) ficam fora do catálogo. A regra está em `ai_provider::opencode_wire`; se a tabela mudar, é ali que se atualiza.
- A prévia de consentimento mostra "OpenCode Zen" ou "OpenCode Go" e o modelo, e diz que o OpenCode repassa os trechos ao provedor do modelo escolhido.

## Consequências

- `application::profile` ganha os dois tipos, campos opcionais da conta ChatGPT e do OpenCode (com `serde(default)` para ler arquivos antigos) e as regras de consentimento por tipo; `ai-provider` ganha o catálogo, o login ChatGPT, o extrator pela Responses API com streaming e o extrator pelo gateway do OpenCode.
- O adapter de captura do OpenCode não muda.
- Configurações › IA ganha a escolha entre quatro provedores, a lista de modelos e o painel de login.
- Entrar com a conta ChatGPT guarda a conta e o token sem trocar o tipo do perfil: o usuário escolhe um modelo do plano e salva para trocar, então um login nunca desliga outro provedor já consentido. Sair revoga o token na OpenAI e apaga o do cofre mesmo se a revogação não for confirmada; o consentimento só cai se o perfil usava a conta.
- Cada execução continua registrando perfil, adapter, modelo e hashes (§12); o tipo novo aparece como adapter.

## Alternativas rejeitadas

- **Reaproveitar o login do Codex CLI** (`~/.codex/auth.json`): não é o fluxo documentado para terceiros e pode violar os termos.
- **Usar o `backend-api` do ChatGPT**: a documentação manda usar só a Responses API pública.
- **Guardar o access token no cofre:** o JWT pode passar do limite de tamanho do Credential Manager e só vale 1 h; basta renová-lo pelo refresh token.
- **OpenCode sem filtro de sessão**: criaria o ciclo captura → extração → captura.
