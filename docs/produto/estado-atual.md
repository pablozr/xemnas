# Estado atual do produto

**Status:** referência de escopo vigente desde 06/10/2026. Substitui o
[MVP-SPEC](../historico/MVP-SPEC.md), que ficou como registro. Em caso de dúvida entre
este texto e o código, vale o código; corrija este texto no mesmo commit.

## O que o xemnas é

Um app desktop local que acompanha o trabalho de agentes de código, propõe as decisões que
apareceram nele, guarda as que você confirma e devolve essas decisões ao agente como contexto.

## O que faz hoje

**Captura.** O plugin do OpenCode e os hooks do Claude Code enviam a sessão à API local
(só loopback, com token). Segredos são mascarados no adapter e de novo no motor. Com o app
fechado, a captura espera numa outbox de arquivos e entra quando o app abre.

**Análise.** Um job em segundo plano lê a captura e propõe candidatos: decisões e regras
(uma regra vira claim). Detalhes de implementação são descartados. Cada candidato traz
relevância, evidências e, quando há, ressalvas de alcance. O extrator padrão é offline. Fatos
de manifests (`Cargo.toml`, `package.json`) viram observações descritivas locais, que nunca
entram na fila de revisão. Documentação do projeto (README, ADRs, `docs/`) é indexada e pode
gerar candidatos.

**Revisão.** Duas formas, por instalação.
- *Manual* (padrão): nada vira decisão sem você. Você confirma, ajusta, adia ou rejeita, com
  desfazer. Ao confirmar, a tela mostra os vínculos com o mapa e você escolhe quais valem.
- *Automático*: regras locais descartam repetições e um juiz de IA decide o resto em lote.
  Tudo aparece em "Feito sozinho", com o motivo, e o que o juiz não soube decidir volta para
  você. Ver [ADR-0012](../arquitetura/adr/0012-modo-automatico-e-autoridade.md).

**Decisões e relações.** Revisar uma decisão cria nova versão e mantém a anterior. Você marca
o que substitui, depende de ou conflita com o quê. A IA pode sugerir relações e regras
derivadas, sempre com citação literal, e nada vale antes de você confirmar. Há busca,
exportação para Markdown ou JSON e uma revisão consultiva, sob demanda, que aponta tensões
entre decisões sem alterar nada.

**Regras com escopo.** Uma regra ligada a componentes só vale quando a tarefa toca um deles.
Sem ligação, vale para o projeto todo (`crates/application/src/graph/scope.rs`).

**Grafo e mapa.** Componentes e tecnologias do projeto, com os arquivos que cada um cobre e as
decisões e regras ligadas a eles. Os componentes vêm do workspace declarado e dos arquivos que
as decisões tocaram. Vínculos propostos esperam sua confirmação. A tela Mapa tem lista, grafo,
lente de arquivo e linha do tempo. A tela Visão mostra um resumo e os fluxos principais
escritos pela IA, cada frase com suas fontes. Há também uma página HTML exportável.

**Contexto para o agente.** Decisões e regras em vigor voltam ao agente de três formas:
- *Injeção* no prompt, dentro de um orçamento de tokens, por projeto: Desligado, Medir (só
  registra) ou Injetar.
- *Depois de editar arquivos*, no OpenCode: só o que o mapa liga aos arquivos editados.
- *MCP* somente leitura: `get_decision`, `search_context` e `file_context`.

A busca é lexical (FTS5) mais o grafo. O Context Pack manual aparece na tela Contexto.

**Projetos e worktrees.** Um worktree do git conta como o projeto registrado
(`crates/application/src/repo_identity.rs`). Edições em outro worktree do mesmo repositório
entram com o caminho relativo a ele (`apps/mcp-server/src/hook/worktree.rs`). Documentos de worktrees
não são varridos: em Contexto, Documentação, "Importar documento…" traz um único arquivo da pasta do
projeto ou de um worktree do mesmo repositório, com o caminho relativo ao worktree dele, e o enfileira
para a extração e a revisão normais (`Documents::import_file`). Só documentação em UTF-8 até 512 KB.

**Interface.** App nativo em Rust + GPUI. Telas: Revisão, Decisões, Contexto, Mapa, Visão,
Projetos e Configurações (IA, OpenCode, Diagnóstico, Aparência, Idioma). Paleta com Ctrl K.
Dez idiomas, inglês por padrão ([idiomas](../design/idiomas.md)). O painel do assistente
mostra a fila real e leva à tela certa; ainda não conversa com um modelo.

## Integrações

- **OpenCode:** plugin em `adapters/opencode`. Captura, valida o envelope e envia ao app ou à
  outbox. Não tem regra de negócio e nunca chama modelo.
- **Claude Code:** `xemnas-mcp hook prompt` injeta contexto, `hook stop` captura os turnos e
  `hook session-end` envia o que ficou retido. O mesmo `xemnas-mcp` serve o MCP. Um hook
  nunca bloqueia nem quebra o Claude Code.
- **Provedores de IA** ([ADR-0004](../arquitetura/adr/0004-provedores-de-ia-conta-chatgpt-e-opencode.md)):
  heurística local, API compatível com OpenAI (inclui modelo local), conta ChatGPT, OpenCode
  Zen/Go e Claude Code local (experimental).

## Dados e privacidade

- Tudo fica em `%LOCALAPPDATA%\xemnas` (SQLite). A API local escuta só em loopback.
- Só sai da máquina o que um provedor externo recebe, depois de uma prévia e do seu
  consentimento. Trocar tipo, destino ou modelo pede novo consentimento.
- Chaves e tokens ficam no Gerenciador de Credenciais do Windows, nunca em arquivo.
- Logs e diagnósticos exportados não levam conversa, diff nem credencial.
- O xemnas não faz commit nem escreve no seu repositório. Exportar é ação sua.

## Medição

- **Diagnóstico** (Configurações): latência, perdas, jobs com nova tentativa, calibração do
  modo automático. Exporta sem conteúdo.
- **`tools/core-quality.py`:** portões de assertividade e desempenho do núcleo
  ([qualidade do núcleo](../arquitetura/qualidade-do-nucleo.md)).
- **Dogfood automático:** `tools/dogfood-report.py` lê o banco local só para leitura e
  preenche o [registro](../operacao/dogfood-log.md) por dia. A tabela `agent_queries` guarda
  cada consulta MCP (ferramenta, resultado e tamanho, sem texto) para medir se o agente usa o
  contexto.
- Orçamentos de desempenho: [desempenho e escala](../arquitetura/desempenho-e-escala.md).

## Limites conhecidos

- Só Windows. GPUI roda em outros sistemas, mas o empacotamento e partes da plataforma não.
- O Controle de Aplicativo Inteligente (Smart App Control) pode bloquear builds novos; veja
  [operação](../operacao/operacao-e-referencia.md#smart-app-control-sac).
- Só o OpenCode e o Claude Code capturam. Outros agentes ainda não têm adapter.
- A busca semântica por embeddings foi medida e reprovada para o contexto
  ([ADR-0013](../arquitetura/adr/0013-contexto-sem-embeddings.md)).
- A biblioteca global de documentos (PDFs, livros) não foi iniciada.
- A confirmação de normas é humana, salvo no modo automático, que você liga por escolha.
- A verificação da assinatura do ID token do ChatGPT está pendente
  ([ADR-0004](../arquitetura/adr/0004-provedores-de-ia-conta-chatgpt-e-opencode.md)).

## Para onde vamos

A direção atual é "core first": menos atrito para quem usa, o grafo como centro da memória e
injeção assertiva (entregar pouco, certo e na hora). Toda mudança no núcleo traz um teste de
desempenho e um de assertividade.

Do plano antigo, ainda valem como direção:
- medir no uso real se o contexto entregue ajuda, antes de ampliar a injeção;
- detectar contradições e condições de reconsideração entre decisões;
- novos adapters para outros agentes;
- uma biblioteca de conhecimento reutilizável entre projetos, citável e nunca tratada como
  decisão do projeto.

Limites que não mudam: o sistema observa, organiza, recupera e aconselha. Ele não inventa
justificativas, não transforma referência global em decisão e não escreve no repositório sem
ação explícita. A confirmação automática só existe quando você a liga.

Pesquisas e ideias abertas: [pesquisas](../pesquisas/README.md). Plano: [roadmap](../roadmap/).
