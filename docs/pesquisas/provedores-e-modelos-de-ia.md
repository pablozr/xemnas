# Escolha de provedor e modelo para a extração

**Status:** Em implementação, conforme `docs/arquitetura/adr/0004-provedores-de-ia-conta-chatgpt-e-opencode.md`.

Pesquisa de 30/09/2026. Parte do que está aqui vem de resumos de busca, porque
`developers.openai.com`, `opencode.ai` e sites de notícia estavam bloqueados no
ambiente da pesquisa. Endpoints e escopos marcados com **(confirmar)** precisam
ser conferidos na documentação oficial antes de implementar.

## Hoje

- Dois modos: `Fake` (filtro de relevância real + texto em modelo fixo) e
  `OpenAiCompatible` (`POST {endpoint}/chat/completions`,
  `response_format: json_object`, chave obrigatória no cofre).
- `grant_consent` exige chave guardada, então um modelo local sem chave
  (Ollama, LM Studio) não consegue ser ativado, embora o endpoint em loopback
  seja aceito.
- O modelo é digitado à mão; não há lista de modelos.
- MVP-SPEC §12 e o ticket 13 escolheram "um provider real de menor esforço" e
  disseram que o OpenCode é orquestrador, não modelo. Usar o OpenCode como
  provedor revisa essa decisão e pede um ADR.

## Opções encontradas

### 1. Conta ChatGPT (usar o plano, sem chave) — recomendado

A OpenAI abriu o **Sign in with ChatGPT** com uso do plano em apps de terceiros.
O acesso comercial é por trial limitado, mas **o uso do plano está disponível para
todos os parceiros open source**, e existe um fluxo específico para apps open
source que rodam localmente, sem client secret nem chave de parceiro. O xemnas se
encaixa: MIT, desktop local, já tem cofre do sistema.

Como funciona, pelos resumos e por implementações abertas que já integraram:

- OIDC com PKCE e redirect em loopback (`http://127.0.0.1:<porta>/auth/callback`).
  Não há variante com código de dispositivo: o navegador abre na própria máquina.
- Registro dinâmico: o primeiro login usa `client_id=dynamic_agent_client` (com um
  `agent_name_hint`) e recebe um client id que deve ser persistido; logins
  seguintes pulam o consentimento. Cada máquina envia um `ext_agent_host_id`
  estável (`urn:uuid`).
- Endpoint de autorização `https://auth.openai.com/api/accounts/authorize`
  **(confirmar)**; token e refresh **(confirmar)**.
- Escopos: `openid profile email` para identidade e os de uso do plano. Só há uso do
  plano se o escopo `chatgpt.tokens.use.direct` voltar concedido; sem ele, o login
  é válido mas a inferência fica desligada.
- Access token de 1 h; refresh token rotaciona a cada uso e fica **só no cofre**.
- Inferência pela **Responses API** (`POST /v1/responses` com `store: false`) e
  lista de modelos por `GET /v1/models`. Base URL para o token do plano
  **(confirmar)**.
- O uso conta no limite do plano (Plus/Pro/Business…); o usuário define teto
  semanal por app em ChatGPT › Configurações.
- Diretrizes de UI: mostrar "usando o plano do ChatGPT" e um link "Gerenciar uso"
  para `chatgpt.com/settings/usage` **(confirmar URL e texto do botão)**.
- Tokens nunca saem da máquina do usuário; gateways hospedados precisam de acordo
  próprio. Isso combina com o produto.

Não confundir com reutilizar o login do Codex CLI (`~/.codex/auth.json`): isso não
é o fluxo documentado para terceiros e pode violar os termos.

### 2. OpenCode como provedor

O OpenCode já é launch partner do Sign in with ChatGPT e conecta 75+ provedores
(OpenAI com plano Plus/Pro, Anthropic, Copilot, OpenRouter, Ollama…). Usar o
servidor do OpenCode que o usuário já tem aberto daria acesso a tudo isso sem
configurar nada no xemnas.

- Modelos identificados como `provider/model`; lista pelos provedores
  configurados no servidor **(confirmar rota: `GET /config/providers` ou `/provider`)**.
- O SDK aceita `format: { type: "json_schema", schema }` no `session.prompt`, e o
  modelo devolve JSON validado pelo esquema.
- Custos: cada extração cria uma sessão no OpenCode do usuário (precisa ser
  apagada depois ou ficar num agente sem ferramentas), depende do OpenCode estar
  aberto e muda a decisão registrada no §12. Pede ADR.

### 3. Modelo local (Ollama, LM Studio)

Já funciona pelo caminho compatível com OpenAI em loopback, faltando:
chave **opcional** para endpoints de loopback, detecção do servidor e lista de
modelos por `GET /v1/models`. Mudança pequena e sem nada saindo da máquina.

### 4. Chave de API (OpenAI, OpenRouter, outros)

É o modo atual, com **predefinições** de endereço (OpenAI, OpenRouter, Groq,
"Outro compatível") e lista de modelos pela mesma rota. Anthropic usa outra API;
fica coberta via OpenRouter ou OpenCode, sem adapter próprio agora.

## Proposta de backend (sessão de backend)

1. **Perfil:** `ProviderKind { Heuristic, LocalModel, ChatGptPlan, OpenCode, ApiKey { preset } }`
   com `model` e `endpoint` onde fizer sentido; leitura do arquivo antigo com
   default (`fake` → `Heuristic`, `open_ai_compatible` → `ApiKey`/`LocalModel`
   pelo host). O hash da prévia passa a incluir provedor e forma de autenticação.
2. **Consentimento:** continua obrigatório para tudo que não é `Heuristic`; a chave
   deixa de ser requisito para `LocalModel` (loopback) e `ChatGptPlan` (usa o login).
3. **Catálogo:** porta `ModelCatalog::list(profile) -> Vec<ModelInfo { id, label }>`,
   implementada em `ai-provider` por `GET /v1/models` e, para o OpenCode, pela rota
   de provedores. Falha vira lista vazia com motivo; digitar à mão continua possível.
4. **Conta ChatGPT:** caso de uso `ChatGptAccount { sign_in, status, sign_out }`.
   `sign_in` sobe um listener em `127.0.0.1` porta livre, abre o navegador, troca o
   código com PKCE, verifica o ID token, grava client id e `ext_agent_host_id` no
   perfil e o refresh token no cofre. `status` devolve e-mail, plano e se o escopo
   de uso veio. `sign_out` revoga e apaga do cofre.
5. **Extrator pela Responses API** com `json_schema` estrito (serve ao plano ChatGPT
   e à OpenAI por chave), reaproveitando a validação de lote atual.
6. **Teste de conexão** (já existe no application) estendido a cada provedor.
7. **OpenCode**: só depois de um ADR revisando o §12.

## Proposta de interface

- **Provedor** vira uma grade de cartões (o componente da tela atual), cada um com
  ícone, nome, uma linha e um selo honesto:
  - Heurística local — "Sem rede · texto modelo"
  - Modelo local — "Na sua máquina"
  - Conta ChatGPT — "Usa seu plano"
  - OpenCode — "Usa o que você já configurou" (quando existir)
  - Chave de API — "Cobrado por uso"
- **Painel do provedor selecionado**, abaixo da grade:
  - Conta ChatGPT: botão **Entrar com ChatGPT** (seguir as diretrizes de marca);
    depois do login, avatar/e-mail, "Usando o plano do ChatGPT", **Gerenciar uso**
    (abre `chatgpt.com/settings/usage`) e **Sair**. Enquanto o navegador está
    aberto, estado "Aguardando o login no navegador…" com **Cancelar**.
  - Modelo local: endereço preenchido com `http://127.0.0.1:11434/v1` (Ollama) e
    atalho para LM Studio (`:1234/v1`); **Detectar** mostra "Encontrado · N modelos"
    ou "Nada respondendo nesse endereço".
  - Chave de API: seletor de predefinição, endereço (editável só em "Outro") e a
    chave mascarada que já existe.
- **Seletor de modelo** em todos os provedores com rede: campo com busca que abre
  uma lista (reaproveitando o popover da paleta de comandos), **Atualizar lista**, e
  digitar um id à mão quando a lista falhar. Mostrar o modelo escolhido também no
  status do topo ("Ativo · gpt-x via plano ChatGPT").
- **Prévia e consentimento** ficam como estão, com a linha "Autenticação: plano do
  ChatGPT de fulano@…" ou "chave no cofre" ou "sem chave (local)".
- A Heurística local ganha um aviso honesto: detecta onde houve decisão, mas o texto
  do candidato é um modelo fixo para você reescrever em **Ajustar**.

## Ordem sugerida

1. Modelo local sem chave + lista de modelos + predefinições (pequeno, tudo local).
2. Conta ChatGPT (maior ganho: extração real sem chave nem custo extra).
3. OpenCode como provedor, depois do ADR.

## Fontes

- [Sign in with ChatGPT — OpenAI Developers](https://developers.openai.com/siwc) e [Quickstart](https://developers.openai.com/siwc/quickstart)
- [Sign in with ChatGPT para apps open source](https://developers.openai.com/siwc/token-sharing-open-source) e [erros e recuperação](https://developers.openai.com/siwc/token-sharing-open-source/errors-and-recovery)
- [Using your ChatGPT plan in other apps and sites — OpenAI Help Center](https://help.openai.com/en/articles/20001542-using-your-chatgpt-plan-in-other-apps-and-sites)
- [Formulário de interesse para parceiros comerciais](https://openai.com/form/sign-in-with-chatgpt-interest/)
- [The New Stack — anúncio no DevDay](https://thenewstack.io/sign-in-with-chatgpt/)
- Integrações abertas usadas como referência técnica: [justrach/harness#129](https://github.com/justrach/harness/issues/129), [zegadb/desktop#27](https://github.com/zegadb/desktop/issues/27)
- [OpenCode — provedores](https://opencode.ai/docs/providers/), [SDK](https://opencode.ai/docs/sdk/), [structured output (anomalyco/opencode#10456)](https://github.com/anomalyco/opencode/issues/10456)
- [OpenCode com plano ChatGPT Plus/Pro](https://agent.space/blog/opencode-chatgpt-subscription)
- Código atual: `crates/application/src/profile.rs`, `crates/ai-provider/src/lib.rs`, `docs/roadmap/mvp/issues/13-perfil-ia-consentido.md`
