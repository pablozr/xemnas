# 21 — Mensagens do backend traduzíveis

**What to build:** Fazer o backend devolver motivos tipados em vez de frases
prontas em português, para a interface mostrar tudo no idioma escolhido
([idiomas.md](../../design/idiomas.md)).

**Blocked by:** nada. É trabalho da sessão de backend; o front só troca a
leitura da mensagem pela tradução do motivo.

**Status: open**

A interface já está em dez idiomas (padrão inglês), mas estas mensagens chegam
prontas de `application`/`ai-provider` e aparecem em português em qualquer
idioma:

- [ ] `application::providers::ProviderError::message()` — falha ao listar modelos e no
  login da Conta ChatGPT (`screens/settings/providers.rs`).
- [ ] `application::profile::ProfileError::Invalid(..)` — erro ao salvar o
  perfil de IA (`screens/settings/mod.rs`, `failure()`).
- [ ] Motivo de bloqueio de `consent_status` fora dos três já mapeados pela
  tela (`blocked_reason`, ramo `other`).
- [ ] `check.message` das verificações do teste de conexão do OpenCode
  (`screens/settings/opencode.rs`).
- [ ] `Display` de `OverviewError` mostrado pela Visão (`screens/overview.rs`,
  `product()`), por exemplo "Ative um provedor de IA em Configurações › IA…".
- [ ] Relatório da revisão de conhecimento: `omission.reason`,
  `unit.selection`, `finding.explanation`, `finding.question`
  (`screens/knowledge_review.rs`).
- [ ] Motivo de `InvalidRelation` ao relacionar decisões
  (`screens/decisions.rs`).

- [x] Motivos do juiz da revisão automática, do proponente de vínculos e do de relações:
  agora no idioma da interface, por `LanguageSource` (ADR-0018, tarefa 1). O que o juiz lê é
  inglês; as razões gravadas e as strings de produto do ledger seguem abaixo.
- [ ] Razões de vínculo gravadas em português: `citado no texto`, `dependência citada`,
  `símbolo citado`, `sugerido pela IA`, `indicado pelo extrator junto com a decisão`
  (`EXTRACTED_LINK_WHY`) e a marca `polaridade duvidosa` (`DOUBT_MARK`); a tela do Mapa e o
  juiz as leem como prefixo. O "em" de `with_doubt` (`"<gatilho>" em "<citação>"`) é
  português dentro da marca; `doubt_parts` o separa para a tela, que mostra a linha traduzida.
- [x] O rótulo "Alerta de polaridade" que o juiz recebe em `link_text`
  (`auto_approval.rs`): agora "Polarity alert", em inglês (ADR-0018, tarefa 1).
- [ ] A Visão guardada não registra o idioma em que foi escrita: trocar o idioma da interface
  não a regera, e a página não sabe dizer em que idioma ela está.

**Aceite:** cada caso vira um enum (ou código estável) no contrato do
`application`; a tela traduz por `crate::i18n`; nenhuma frase em português
sai do backend para a interface. Texto destinado a agentes (MCP,
`agent_access`) não entra neste ticket.
