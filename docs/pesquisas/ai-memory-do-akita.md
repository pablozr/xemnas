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

## O que trazer

Em ordem de custo/benefício; todas sem chamada de IA nova nem dependência nova.

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

**Recomendação:** trazer as ideias 1 → 2 → 3 → 4 → 5 depois do dogfood; integrar só se houver
demanda real, começando pelo exportador.

## Não conferido

- O fio de respostas do post (lido só o texto principal, por espelho) e o changelog da v2.6.
- Números de recuperação de `docs/benchmarks/` dele e o tamanho do resumo por projeto.
- A lista completa de ferramentas MCP (só as três acima). (confirmar)
