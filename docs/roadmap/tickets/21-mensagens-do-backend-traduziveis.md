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

**Aceite:** cada caso vira um enum (ou código estável) no contrato do
`application`; a tela traduz por `crate::i18n`; nenhuma frase em português
sai do backend para a interface. Texto destinado a agentes (MCP,
`agent_access`) não entra neste ticket.
