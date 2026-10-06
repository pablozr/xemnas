# Como implementar o Procurador e o mapa de cegueira no Xemnas

**Data:** 06/10/2026.
**Pergunta:** onde o [Procurador](procurador.md) e o [mapa de cegueira](mapa-de-cegueira.md)
entram no código atual, em que ordem, com que contratos novos e como provar cada etapa?
**Status:** Aberta. Estudo de implementação sobre o código de `research/dreaming`
(`c7702be`); nada implementado.

## Correções ao desenho, vindas de uma medição local

Uma leitura dos transcripts do Claude Code do usuário (autorizada, local, só agregados; os
números ficam fora do repositório) mudou duas coisas no Procurador:

- **A maioria das paradas é pedido de licença**, não falta de decisão: o agente para para
  perguntar se segue a própria recomendação, se faz commit, push ou registra um ticket. Por
  isso entra uma terceira fonte de resposta, a **política de autonomia**, além de decisões e
  regras.
- **Uma pergunta coberta por regra pode ser um pedido de exceção**, e a pessoa às vezes a
  concede. Regra do desenho: pedido de exceção a uma regra conhecida vai sempre para a pessoa.

E uma ao mapa: quando a pessoa diz que não entendeu e pede explicação, isso é a marca mais
forte de lacuna (e de que ela está sendo fechada).

## O que já existe e será reaproveitado

| Peça | Onde | Uso |
| --- | --- | --- |
| Gancho `Stop` que já lê o transcript por turno | [stop.rs](../../apps/mcp-server/src/hook/stop.rs), [transcript.rs](../../apps/mcp-server/src/hook/transcript.rs): `Turn` com `user_text`, `assistant_texts`, `tools`, `edits` | Ver a última mensagem do agente e mandar a pergunta ao app |
| Gancho do prompt com chamada ao app e timeout de 300 ms | [prompt.rs](../../apps/mcp-server/src/hook/prompt.rs) → `POST /v1/context` | A próxima fala da pessoa resolve a pergunta pendente, sem gancho novo |
| Turno trivial dobrado no anterior | `Turn::is_trivial`, `group` em `stop.rs` | Um "pode seguir" já chega junto da pergunta na mesma captura |
| Modos por projeto `Off`/`Shadow`/`Inject` | [context_settings.rs](../../crates/application/src/context_settings.rs): `ContextMode` | Mesmo padrão para o modo do Procurador |
| Calibração por AUC com mínimo de casos | [calibration.rs](../../crates/application/src/calibration.rs): `MIN_DECIDED`, `PREDICTS_AT` | Portão para sair da sombra |
| Seleção de contexto, ponte PT/EN, grafo | [context.rs](../../crates/application/src/context.rs), [terms.rs](../../crates/application/src/terms.rs), [injection.rs](../../crates/application/src/injection.rs) | Casar a pergunta com decisões e regras |
| Regras tipadas (`Convention`, `Constraint`…) e sugestões de regra | `ClaimKind` no domínio; [claim_suggestions.rs](../../crates/application/src/claim_suggestions.rs) | A política de autonomia é uma `Convention` proposta e revisada |
| Arquivo → componente | `components_for` em [graph/query.rs](../../crates/application/src/graph/query.rs) | Agregar a cegueira por componente |
| Redação de conteúdo | [redact.rs](../../crates/application/src/redact.rs) | Antes de gravar qualquer pergunta ou resposta |

## A peça comum: sinais de turno

Os dois recursos precisam classificar falas da pessoa e do agente. Um módulo puro,
`application::turn_signals`, sem armazenamento nem IA:

- `classify_question(text, options) -> Option<Question>`: a última mensagem do agente é uma
  pergunta à pessoa? De que tipo: **licença** (com categoria fechada), **escolha** (opções),
  **exceção** (cita uma regra e pede para contorná-la) ou nenhuma (retórica, relato, aviso de
  limite).
- `classify_reply(text) -> Reply`: **aprovação**, **aprovação com acréscimo** (o acréscimo
  separado, candidato a regra), **escolha** de uma opção, **redirecionamento** ou
  **pedido de explicação** ("não entendi", "explique melhor").
- `classify_turn(prompt, edits) -> Engagement`: **explicado**, **supervisionado** ou
  **às cegas**, pelo prompt que abriu o turno das edições.

Categorias de licença, fechadas e com risco fixo:

| Categoria | Exemplo | Pode virar resposta automática? |
| --- | --- | --- |
| `continue_plan` | "Sigo com o que recomendei?" | Sim, com política confirmada |
| `record_doc` | "Registro como ticket ou pesquisa?" | Sim, com política confirmada |
| `commit_local` | "Faço o commit?" | Sim, com política confirmada |
| `push_work_branch` | "Dou push na branch?" | Sim, com política confirmada |
| `push_protected`, `merge`, `open_pr` | `master`, merge, PR | Nunca |
| `install_or_download`, `destructive`, `external_message` | baixar, apagar, publicar | Nunca |

Lista fechada porque precisão vale mais que cobertura: o que não cair numa categoria segura
vai para a pessoa.

## Procurador

### Fluxo

1. **Stop** (gancho). Depois da captura que já faz, o `stop.rs` olha o último turno lido. Se a
   última mensagem do agente termina em pergunta, manda `POST /v1/proxy/ask` com projeto,
   sessão, `prompt_uuid`, o último parágrafo (até 1.000 caracteres, redigido no app), opções
   e arquivos recentes. Timeout de 300 ms, como o do prompt.
2. **App** (`application::proxy`): classifica a pergunta e procura resposta, nesta ordem:
   - categoria nunca-automática, exceção ou pergunta repetida no mesmo turno → `forward`;
   - licença com **política de autonomia** confirmada para a categoria → `answer`;
   - escolha coberta por **uma única** decisão ou regra em vigor, no escopo, sem conflito
     aberto e acima do limiar → `answer`, citando `D:ref vN`;
   - senão → `forward`.
   Grava a pergunta pendente no livro do Procurador, com o que teria respondido.
3. **Resposta** (só no modo ativo): o gancho imprime `{"decision":"block","reason":"…"}` e o
   agente continua lendo o motivo. Proteção contra laço: no máximo uma resposta por
   `prompt_uuid`, registrada no checkpoint da sessão que o `stop.rs` já mantém (a
   documentação sugere `turn_number`; conferir no payload real).
4. **Fechamento** (gancho do prompt): o `/v1/context` recebe a próxima fala da pessoa na mesma
   sessão; se há pergunta pendente, `classify_reply` a resolve e grava o desfecho: acerto ou
   erro da sombra, aprovação simples, acréscimo ou redirecionamento. Nenhum gancho novo.
5. **Depois:** `AskUserQuestion` via `PreToolUse` (resposta em `permissionDecisionReason`) e
   OpenCode via `session.idle`.

### Política de autonomia

- **Estatística por categoria** no livro: pedidas, aprovadas, aprovadas com acréscimo,
  redirecionadas.
- **Proposta**: com N pedidas e todas aprovadas (N a fixar, começando em 4), entra na fila de
  revisão uma `Convention` do tipo licença ("Quando terminar uma mudança validada, faça
  commit local sem perguntar"), com as perguntas e respostas como evidência.
- **Efeito duplo**: confirmada, ela entra no **contexto injetado** (o agente deixa de parar) e
  o Procurador passa a responder a categoria, se o agente perguntar mesmo assim.
- **Acréscimos** ditos junto com uma aprovação viram candidatos a regra pela extração que já
  existe; o livro só marca a origem.

A política injetada pode resolver boa parte da dor **sem resposta automática nenhuma**. Medir
isso primeiro: a taxa de paradas por sessão antes e depois de injetar as políticas.

### Dados e contratos novos

- Tabela `proxy_ledger`: `project_id`, `session_id`, `prompt_uuid`, `asked_at`, tipo e
  categoria, pergunta redigida (curta), itens candidatos (`D:ref vN`, regra ou política),
  resposta que daria, modo, desfecho (`answered`, `forwarded`, `shadow_hit`, `shadow_miss`,
  `approved`, `approved_with_addendum`, `redirected`), `resolved_at`. Migração nova.
- `proxy_settings` por projeto: `off`, `shadow` (padrão ao ligar), `active`.
- Endpoint `POST /v1/proxy/ask` na API local, autenticado como o `/v1/context`.
- A resposta da pessoa **não** é guardada inteira: só a classificação e, no acréscimo, o trecho
  redigido que vira candidato (a captura já guarda o texto pelo caminho normal).
- **ADR antes do modo ativo**: o Procurador aplica autoridade da pessoa (combinado com o
  usuário em 06/10/2026). O ADR fixa as categorias nunca-automáticas, a regra de exceção, o
  limiar e o desfazer.

## Mapa de cegueira

### Fluxo

1. **Na ingestão** da captura ([captures.rs](../../crates/application/src/captures.rs)):
   `classify_turn` sobre o `user_text` e os `diff_hunk` da captura grava, por arquivo, linhas
   adicionadas e a classe. Custo: um passe linear sobre o que já foi lido; sem IA.
2. **Marcas de explicação**: `classify_reply` com pedido de explicação marca os componentes
   citados na fala como "em explicação"; uma decisão confirmada no componente o marca como
   explicado a partir daquela data.
3. **Agregado sob demanda**: `blindness_map(project, janela)` junta arquivos em componentes
   pelo `components_for` atual (o mapa muda; por isso a agregação é na leitura, com cache
   invalidado na atualização do mapa) e devolve, por componente: linhas por classe, sessões,
   sessões de correção e uma nota de prioridade (cegueira × movimento × dor × centralidade).
4. **"Vivo"** começa aproximado: linhas da janela (90 dias), menos as reescritas por capturas
   posteriores no mesmo arquivo. `git blame` só sob demanda, para os componentes do topo.

### Dados e contratos novos

- Tabela `capture_engagement`: `capture_id`, `path`, `added_lines`, `class`, `fix_session`.
  Migração nova. Nenhum texto, só classe e contagem.
- Consulta no `application` para a tela; campo opcional no `file_context` do MCP
  ("a pessoa não discutiu este módulo") só depois do teste falsificável e sem baixar a precisão
  do contexto nos portões.

## Ordem de entrega

Cada etapa é um commit pequeno, com portão de assertividade e de desempenho
(`tools/core-quality.py`), e pode parar a ideia se o número não vier.

1. **`turn_signals` + corpus às cegas + portão.** Corpus sintético de perguntas e respostas
   (licença por categoria, escolha, exceção, ruído; aprovações, acréscimos, redirecionamentos,
   pedidos de explicação) e de turnos com diff. Holdout selado. Microbenchmark por chamada.
2. **Mapa, backend**: `capture_engagement` na ingestão e `blindness_map`. Teste falsificável:
   zonas cegas têm sessões de correção mais longas? Script local sobre o `app.db` do dogfood
   (só leitura, já liberado) quando houver captura suficiente. Sem correlação, o mapa para
   aqui.
3. **Procurador em sombra**: `/v1/proxy/ask`, `proxy_ledger`, fechamento pelo `/v1/context`,
   métricas na exportação de diagnóstico. Nenhuma saída para o agente.
4. **Políticas de autonomia**: estatística, proposta na revisão, injeção. Medir paradas por
   sessão antes e depois.
5. **ADR de autoridade** e **modo ativo** no `Stop`, com limiar calibrado sobre o livro da sombra.
6. **Front** (sessão de front): linha na Revisão ("respondi N, 0 min de espera, 1 desfeita"),
   configuração do modo, lente de cegueira no Mapa, linha na Visão, tour de 3 minutos.
7. `AskUserQuestion`, OpenCode e o sinal de cegueira no `file_context`.

## Fronteiras de sessão

O gancho mora em `apps/mcp-server`, que não é tela: a mudança no `stop.rs` pertence à sessão de
backend, mas deve ser avisada como mudança em `apps/`. Telas ficam com a sessão de front, que só
chama casos de uso do `application`.

## Riscos

- **Classificação de pergunta em PT e EN** com falso positivo de licença: mitigado pela lista
  fechada, pelo corpus com negativas de mesmo vocabulário e pela sombra.
- **Laço de Stop**: uma resposta por `prompt_uuid`; pergunta repetida vai para a pessoa.
- **Latência no Stop**: o gancho já faz captura; a chamada nova tem teto de 300 ms e falha em
  silêncio, como o prompt.
- **Privacidade**: o livro guarda pergunta redigida e classes, nunca a resposta inteira; o mapa
  guarda só contagens. Tudo local.
- **Mapa como métrica de pessoas**: fica proibido por desenho (componentes, nunca autores).
