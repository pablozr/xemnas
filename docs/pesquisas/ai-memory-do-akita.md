# ai-memory (Akita): o que aproveitar e se vale integrar

**Data:** 07/10/2026.
**Pergunta:** o ai-memory, projeto de memória para agentes do Fabio Akita, tem ideias ou
implementações que o Xemnas deve trazer, e vale abrir um PR de integração entre os dois?
**Status:** Aberta. Estudo de referência; nada implementado. Features congeladas até o fim do
dogfood ([ticket 20](../roadmap/tickets/20-dogfood-e-conclusao.md)).

## O projeto

- Repositório [akitaonrails/ai-memory](https://github.com/akitaonrails/ai-memory), licença
  MIT, Rust. Em 07/10/2026: cerca de 9 mil estrelas, 600 forks, criado em 21/05/2026, releases a
  cada poucos dias (v2.6.0 em 07/10).
- O [post de lançamento da v2.6](https://x.com/AkitaOnRails/status/2107840184133755178)
  anuncia o **perfil global entre projetos**: hábitos que valem para todo projeto ("como
  sempre começamos projetos Rust?"), em `docs/design-cross-project-profile.md` do repositório.
- **Armazenamento:** `wiki/` em markdown versionado no git é a fonte da verdade; `raw/` guarda
  trechos sanitizados de transcript; `db/` é SQLite com FTS5, entidades e embeddings opcionais.
- **Fluxo:** captura → consolida → recupera → handoff.
  - Captura por hooks de ciclo de vida de cerca de 20 agentes (Claude Code, Codex, OpenCode,
    Cursor, Gemini CLI, Copilot CLI, Kiro…), sanitizada antes de gravar.
  - Recuperação por um resumo limitado no SessionStart (3.000 bytes no perfil) e ferramentas
    MCP (`memory_query`, `memory_read_page`, `memory_write_page`). A busca funde FTS5,
    entidades e vizinhos do grafo por RRF, com vetores opcionais.
  - Handoff tipado entre agentes, reivindicado uma única vez.
- **Custo:** nenhuma chamada de LLM por padrão. Envelhecimento sem LLM: meia-vida por tipo,
  reforço por acesso, compressão extrativa por regex; dedup e contradição por embeddings
  (opcionais); reescrita "dream" opcional em ociosidade.
- Servidor HTTP local ou de homelab, com tokens, OIDC, auditoria e UI web somente leitura.

## Comparação

| | ai-memory | Xemnas |
| --- | --- | --- |
| Unidade | Páginas de wiki e observações | Decisões e claims versionadas, revisadas |
| Fonte da verdade | Git + markdown (SQLite é índice) | SQLite (FTS5) |
| Agentes | ~20 | OpenCode e Claude Code |
| Entrega | Resumo no início da sessão | Bloco por pedido (~300 tokens), por tarefa e arquivo, sem repetir |
| Escopo | Projeto, workspace e global | Projeto, com regras por componente |
| Revisão | Opcional | Centro do produto (manual ou juiz em lote) |
| Plataforma | Linux, macOS, Windows | Windows |

Onde ele é melhor: alcance de agentes, handoff, memória entre projetos, envelhecimento
automático, multiplataforma e comunidade. Onde o Xemnas se diferencia: curadoria humana,
decisões versionadas com relações, injeção por tarefa e arquivo com orçamento menor, prévia e
consentimento antes de enviar, app nativo. Em uma linha: o ai-memory lembra de tudo e deixa
envelhecer; o Xemnas guarda pouco e certo.

## Como ele captura e extrai (leitura do código, commit `237933b`)

Caminhos relativos ao repositório dele.

**Captura (sem IA no hook, meta de 200 ms).**
- **Assistente e diffs:** a resposta final do assistente é descartada por padrão; só volta com
  dois opt-ins (2 KB). O raciocínio (thinking) nunca entra. Ferramentas viram só família, desfecho
  e 2 KB de saída. Comando e diff não são guardados (`ai-memory-hooks/src/payload.rs`). O Xemnas,
  ao contrário, manda o turno inteiro com diffs.
- **Desfecho de ferramenta:** só é preenchido quando o protocolo o prova. No Claude Code e no
  Codex fica `Unknown` (`capture_policy.rs:252-316`).
- **Spool:** um arquivo por evento, com TTL de 7 dias e teto de 10 mil. A chave de idempotência
  é gerada uma vez no spool. Falha de rede não gasta tentativa (`commands/hook_spool.rs`).
- **Segredos:**
  - Só regex, sem entropia, e redigir antes de truncar (bug #980).
  - Um tipo `Sanitized<T>` que o writer exige (`core/sanitize.rs`).
- **Anti-eco:** o contexto que ele mesmo injetou leva um marcador e não é recapturado.

**Extração (consolidação com IA, no fim da sessão ou sob demanda).**
- **Tipos:** `rule | decision | gotcha | procedure | fact`. `gotcha` é "modo de falha ou
  surpresa", com causa e mitigação (`ai-memory-consolidate/src/types.rs:96-135`). Não há campo
  para o porquê nem para alternativas rejeitadas: o porquê é prosa no corpo da decisão.
- **Unidade:** a sessão inteira, não o turno.
- **Projeção antes do modelo:** uma projeção determinística pontua as observações e o modelo vê
  as 256 melhores, até 3.000 caracteres cada (`projection.rs:393-470`).
  - Por tipo, com o prompt do usuário acima de tudo.
  - Mais a primeira e a última observação.
  - Mais 45 pontos para termos de sinal: "root cause", "fix", "failed", "regression",
    "decided", "always", "never".
- **Prompt (`prompts/batch_consolidate_system.md`):**
  - Fidelidade às observações.
  - **Proibido listar alternativas que não foram consideradas.**
  - **Preservar a redação do usuário** em decisões e regras.
  - Sessão sem nada durável devolve só a página da sessão.
  - Arestas `causes | fixes | contradicts` só quando a evidência diz isso claramente.
- **Validação:** só estrutural. **A citação não é conferida contra a fonte** (só não pode ser
  vazia, `auto_improve.rs:1805`), e a proveniência para na sessão.
- **Revisão:** aprovação automática por padrão, com piso de confiança 0,75 e sessão de pelo
  menos 8 observações e 120 s. **Memória de rejeições:** até 50 recusas dos últimos 180 dias
  entram no prompt, para não repetir o padrão sem evidência nova (`auto_improve.rs:1191-1295`).
  **Filtros negativos nomeados:** falha transitória de setup, smoke test, marcador de release e
  "a ferramenta X está quebrada", que endureceria recusas futuras.
- **Problemas enfrentados:** nenhum sinal objetivo. Teste que falhou e depois passou, revert,
  erro de ferramenta ou correção do usuário não são detectados. Fica tudo para a palavra-chave e
  o modelo.
- **Força de crença:** contada na leitura por **sessões distintas** que sustentam a página, com
  peso por recência e teto abaixo de 1. Supersessão sempre vence (`ai-memory-store/src/belief.rs`).
- **Atalho de zero tokens:** se o próprio agente já escreveu a página da sessão, o servidor não
  chama IA (`consolidator.rs:444-468`).

**Recuperação.**
- **Puxada, não empurrada:** o hook de prompt não injeta nada. O resumo do início da sessão é
  estático (regras e páginas fixadas), vem desligado e tem orçamento de 4.000 caracteres.
- **Ranking:**
  - FTS5 com `remove_diacritics 2`, entidades, vizinhos do grafo e vetores opcionais, fundidos
    por RRF (k = 60).
  - Depois, um multiplicador de autoridade entre 0,55 e 1,5: tipo regra/decisão acima, sessão
    abaixo, substituída com teto 0,65.
- **Decaimento:** não entra no ranking, só na varredura de esquecimento.
- **Avaliação:** LongMemEval-S fora do CI. Com FTS, hit@5 de 0,67; com embeddings locais, 0,82,
  a cerca de 90 ms e 87 MB a mais. Não há métrica de precisão nem de contaminação.

**Comparação na extração.** O Xemnas é mais rigoroso em três pontos: citação literal conferida,
proveniência até o artefato do turno e revisão antes de virar memória. O ai-memory tem duas
coisas que faltam ao Xemnas:
- um tipo para **problemas enfrentados** (`gotcha`). No Xemnas, uma correção de bug é `detail`
  e é descartada;
- a **contagem de sessões** que sustentam uma memória.

Nenhum dos dois detecta problemas por sinal objetivo.

## O que trazer

Em ordem de custo/benefício; todas sem chamada de IA nova nem dependência nova.

**Na extração** (o pedido original desta pesquisa):

- **A. Duas regras de prompt.** Não inventar alternativas que não aparecem no turno e preservar
  a redação do usuário em decisões e regras. Custo zero. Medir no corpus de extração antes.
- **B. Tipo "problema enfrentado".** Hoje uma correção de bug é `detail` e se perde.
  - Proposta: um `gotcha` com sintoma, causa e mitigação, com citação literal como hoje, que
    entra no contexto quando a tarefa toca o mesmo arquivo ou componente.
  - É mudança de produto: depende do dogfood e de corpus próprio.
- **C. Sinais objetivos de problema no turno**, sem IA:
  - erro de ferramenta;
  - teste que falha e depois passa;
  - revert;
  - "não, faça X" do usuário logo depois de uma resposta.

  Os sinais servem para pontuar o turno antes do extrator e para alimentar B. Casa com
  `application::turn_signals`, já desenhado em
  [implementacao-procurador-e-mapa.md](implementacao-procurador-e-mapa.md). Seria o ponto em
  que o Xemnas passa o ai-memory.
- **D. Filtros negativos nomeados no extrator e no juiz.** O juiz já recebe o gosto do usuário
  (confirmados e rejeitados); o que falta são os filtros com nome.
- **E. Contagem de sessões que sustentam uma decisão**, como reforço de confiança e da ordem no
  pacote. A supersessão sempre vence.

**Na entrega e na manutenção:**

1. **Bloco estável com aviso de excedente.** Mesma entrada, mesmo texto (ordem e formato
   determinísticos), para preservar o cache de prompt do agente; quando sobram itens, uma linha
   "há N itens; consulte `search_context`". O orçamento e a contagem de omitidos já existem em
   `application::injection`. Esforço baixo.
2. **Envelhecimento por meia-vida e reforço por uso.** Ponderar a seleção do Context Pack pela
   idade e pelo uso real (`agent_queries`, `context_injections`). Alimenta
   [sonho-consolidacao-da-memoria.md](sonho-consolidacao-da-memoria.md). Fica de fora a parte
   por embeddings, reprovada em [busca-semantica-local.md](busca-semantica-local.md). Esforço
   baixo a médio; precisa de portão no corpus antes de entrar.
3. **Perfil entre projetos sem LLM.** Uma regra confirmada que aparece em dois ou mais
   projetos (tópico normalizado: minúsculas, sem stop words, nomes de ferramenta preservados)
   vira sugestão de regra global, que passa pela revisão como qualquer outra. Esforço médio.
4. **Ingestão idempotente e checklist de novos agentes.** A chave de idempotência e o dono da
   captura (para dois produtores não capturarem em dobro) e o protocolo
   `docs/managed-harness-contributions.md` dele servem de checklist ao
   [ticket 22](../roadmap/tickets/22-instalacao-e-integracoes-com-um-clique.md). Esforço baixo.
5. **Regras promovidas a um bloco gerenciado do AGENTS.md.** Teto de regras e linhas, hash do
   conteúdo gerado para nunca sobrescrever edição manual. No Xemnas, só como exportação explícita
   feita pela pessoa, porque o app não escreve no repositório. Esforço médio.

## Integração

- Pontos de extensão dele: `POST /hook/batch` para produtores externos, API `/api/v1` e o MCP
  (`memory_write_page` com escopo).
- O caminho natural não exige PR: um **exportador opcional no Xemnas** que grava as decisões
  confirmadas como páginas `decisions/` (que o README dele trata como fonte de alta confiança).
  PR no repositório dele só depois, como exemplo de produtor documentado.
- **Agora não:** os dois capturam pelos mesmos hooks (captura e custo em dobro), seria um
  segundo servidor residente, o ritmo de releases dele quebra integrações e dilui o diferencial.
- Licença MIT: ideias livres; copiar código exige manter o aviso.

**Recomendação:** depois do dogfood, A e D primeiro (só prompt, medidos no corpus), C junto
com `turn_signals`, depois B e E; na entrega, 1 → 2 → 3 → 4 → 5. Integrar só se houver demanda
real, começando pelo exportador.

## Não conferido

- O fio de respostas do post (lido só o texto principal, por espelho) e o changelog da v2.6.
- Números de recuperação de `docs/benchmarks/` dele e o tamanho do resumo por projeto.
- A lista completa de ferramentas MCP (só as três acima). (confirmar)
