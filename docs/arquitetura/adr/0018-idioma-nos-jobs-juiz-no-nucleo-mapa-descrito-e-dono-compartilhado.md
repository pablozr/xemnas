# ADR-0018 — Idioma nos jobs, juiz no núcleo, mapa descrito ao extrator e dono compartilhado

**Status:** Vigente; emenda os itens 5 e 7 do [ADR-0017](0017-polaridade-como-alarme-componentes-na-extracao-e-idioma-da-saida.md).
Aceito em 09/10/2026.

## Contexto

A primeira rodada real no `cloudrs` mostrou quatro defeitos que o ADR-0017 não cobria:

1. Os motivos do juiz saíam em português, espanhol ou inglês, misturados, e os do proponente de
   vínculos também em francês e espanhol.
2. O juiz automático só rodava quando a tela de Revisão era aberta.
3. O extrator propunha poucos componentes: nenhuma das 44 candidatas de documento citou um.
4. Uma dependência declarada por dois componentes ligava a decisão aos dois.

Este ADR cresce por tarefa; cada seção abaixo vale desde o commit que a introduziu.

## Idioma da saída nos jobs

**Causa.** O prompt do juiz é inglês e pede "o idioma do item", mas o conteúdo do usuário ia
todo com rótulos em português (`[vínculo …]`, `Evidência (…)`, `Escolha`, `Componente`,
`Caminhos`, `Alerta de polaridade`, `Pergunta`, `Motivo`, `Confiança do extrator`, `Fontes`,
`Parecidas em vigor`, `## Regras em vigor`, `## Decisões em vigor`, `[relação`, `[contexto`,
`Citação`, `Da decisão`) e as razões gravadas (`dependência citada:`, `; também declarada por
outro componente`) entravam cruas. O modelo seguia o idioma dominante do texto, e no banco real
respondeu até em espanhol. O proponente de vínculos e o de relações tinham o prompt e o pedido
100% em inglês e a instrução vaga "no idioma da decisão", que não ancora nada: saíram francês,
espanhol e português por lote.

**Decisão.**

- `application::output_language::LanguageSource` (`Arc<dyn Fn() -> OutputLanguage>`) é lida
  quando o job roda, não gravada em configuração: segue a troca de idioma ao vivo, não exige
  migração e o `application` não conhece a interface. `LinkFinder`, `RelationFinder` e
  `Approvals` ganham `.with_language(source)` (padrão inglês, então testes e construtores
  existentes continuam valendo). A composition root (`main.rs::interface_language`) passa
  `i18n::current()`.
- Os quatro pedidos (vínculos, relações e os dois do juiz) começam com `Output language: X` e
  os prompts mandam escrever "no idioma de saída nomeado na primeira linha da mensagem"
  (o padrão da Visão: `output_language::header`).
- O texto que o juiz lê é todo inglês (rótulos, tipos de evidência, o alerta de polaridade e a
  montagem da mensagem em `review_message`); só o conteúdo do usuário (pergunta, escolha,
  citação) segue como foi escrito. O formato das **razões gravadas** não muda: o juiz recebe
  uma descrição em inglês da evidência, não a razão crua.
- Strings de produto em português no ledger ("outro item", "decidido pela IA", "a IA não
  respondeu…", os `Triage::Accept("…")`) ficam para o ticket 21.

**Não verificável aqui:** o idioma real das respostas só aparece numa nova rodada no `cloudrs`.

## Juiz disparado pelo núcleo

**Antes.** `Approvals::run` só era chamado por `inbox.rs` (ao trocar de modo e a cada página
da Revisão): com a tela fechada nada era julgado.

**Decisão.**

- `JobKind::AutoReview` (`auto_review`) na fila `suggestions` (1 worker: nunca dois juízes
  juntos), prioridade 5, a mais baixa: roda depois que vínculos, relações, regras e termos
  esvaziaram e cobre tudo numa passada.
- `queue_review_after` é chamada pelo observer de jobs em `main.rs` (uma linha) quando termina,
  com sucesso, `analyze_capture`, `analyze_document`, `suggest_links`, `suggest_relations` ou
  `derive_claims`, o que `JobKind::feeds_review` diz (`match` exaustivo ao lado de `lane`).
  Não enfileira em modo manual nem se já há um `auto_review` na fila: `insert_unless_queued`
  faz as duas coisas num só `INSERT ... WHERE NOT EXISTS` (índice `state`), sem varrer a tabela
  por evento e sem a corrida entre conferir e inserir. Devolve `Result<bool>` (enfileirou) e o
  `main.rs` registra o erro com `tracing::warn!`. Os quatro produtores não mudaram.
- O job percorre todos os projetos (`Approvals::run_all_at`), porque o observer não sabe qual.
  Com o já revisado excluído, uma passada sem nada novo custa algumas leituras e nenhuma
  chamada: `latency auto_review_idle_ms` fica em 6 a 17 ms com 3 projetos.
- Tela e job compartilham **uma** instância de `Approvals` (o `main.rs` a constrói uma vez, com o
  limiter e o idioma, e a entrega à tela), e o `Mutex` é um campo dela, não um `static`: nunca
  julgam juntos e pedem à IA pelo mesmo limiter. O modo é lido **depois** de pegar o lock. A
  tela (`ApprovalsApi::run`) usa `try_lock` e, com uma passada em curso, devolve um relatório
  vazio na hora, porque o job cobre os mesmos itens; o job (`run_all`) espera. O `review_pass`
  da tela fica, redundante e seguro.
- Quando a passada aceita candidatas, a adoção cria vínculos novos que ela não viu: roda uma
  segunda passada (no máximo), **só sobre os vínculos**, para não drenar o resto da fila fora da
  cadência.
- Falha do provedor devolve o job à fila em 20 min (`JobFailure::Deferred`, até 8 vezes). A
  pausa de 20 min que segue uma chamada falha também não termina o job em silêncio: se sobraram
  itens que precisavam do juiz e a pausa os impediu, `RunReport::deferred` traz o que resta
  dela e o job volta à fila para esse momento. Outro erro (armazenamento, por exemplo) termina
  o job como `Failed`, para aparecer no diagnóstico (antes virava `Ok`).

**Lacunas e pedidos ao front.** O refresh disparado pela tela do Mapa não aciona o job (a
Revisão cobre quando é aberta). Pedir ao front: simplificar `review_pass` em `inbox.rs` e dar
nome i18n ao tipo `auto_review` em `diagnostics.rs` (hoje cai em "auto review").

## Mapa descrito ao extrator

**O que os dados reais permitiram concluir.** No `cloudrs` (`state/app.db`, só leitura), as 44
candidatas são todas de documento, o mapa (10 componentes) existia antes da extração (componentes
15:14:50, candidatas a partir de 15:15:38) e nenhuma citou um componente. A premissa de que o
nome precisaria ser normalizado estava errada: `resolve_components` já casava por `entity_key`
(caixa, hífen, espaço e crases) e a citação já usava `quote_matches`. O que os dados **não**
permitem separar é qual das causas prováveis pesou, porque a resposta bruta do modelo não é
guardada de propósito:

1. o extrator recebia só os nomes: sem descrição, ele não tem como saber que "updater/release
   channel" é `sc-platform` ("OS integration: … tray icon and updates");
2. o prompt dizia "never just a word they share", o que pode inibir até citar o nome;
3. a citação pode vir do documento-fonte em vez do texto da candidata.

**Decisão.** Atacar as três e medir, em vez de provar uma:

- a lista do extrator traz `- nome: descrição` (`short_description_for_model`, o mesmo helper do
  pedido de vínculos: descrição de `description_for_model` cortada em 100 caracteres, uma
  linha; 60 cortaria justamente "updates" em `sc-platform`). A descrição que só repete o nome é
  omitida e fica `- nome`;
- o prompt passa a dizer que a parte pode ser nomeada **ou só descrita pelo que faz**, que a
  citação sai do texto que o modelo escreveu para o item e não da fonte, e que palavra comum aos
  dois não conta; mantém "at most 3";
- o nome casa também por apelido (`MapComponent::keys`) e por prefixo antes de `::` ou `/`
  (`sc-platform::update`). Um espaço **não** corta o nome: `sc-platform crate` continua
  desconhecido (o plano pedia as duas coisas, que se contradizem; vale o caso de teste).
- cada análise grava contagens, sem texto: `components_listed` (tamanho do mapa mostrado),
  `components_proposed`, `components_kept`, `components_unknown` (nome fora do mapa) e
  `components_unquoted` (nome do mapa, citação que o texto não sustenta). Repetição e o que passa
  de 3 entram só em `proposed`. Itens malformados que o adaptador descarta em `read_components`
  não são contados (limitação). Linhas antigas ficam `NULL`. As cinco colunas vão no mesmo
  `INSERT` da análise (`AssessmentRecord::components`), sem `UPDATE` depois; as análises que
  falham antes de resolver os componentes deixam `NULL`.

**Como ler o diagnóstico** depois da próxima rodada:

```sql
SELECT components_listed, components_proposed, components_kept,
       components_unknown, components_unquoted FROM assessments;
```

`proposed = 0` com `listed > 0`: o modelo não nomeia (prompt ou descrição); `unknown` alto: ele
inventa ou abrevia nomes; `unquoted` alto: ele cita a fonte ou parafraseia; `kept` próximo de
`proposed`: o recall é do modelo, não da validação.

**Custo.** O prefixo do mapa foi de 1.440 para 7.560 bytes com 60 componentes de descrição cheia
(100 caracteres), teto do gate 8.192. É um prefixo idêntico entre capturas, então o cache do
provedor o paga uma vez. Descrições menores custam menos.

**Rodada real no `cloudrs` (09/10/2026).** Em 12 análises, 54 componentes propostos, 1 mantido e
46 desconhecidos: o modelo copiava a linha inteira, "nome: descrição", e a chave inteira não casava
com nenhum componente. As escolhas e as citações estavam certas. Correção: o cabeçalho e o prompt
pedem só o nome, e `find_component` corta no primeiro `:` (e depois em `/`) quando a chave inteira
não casa; o cabeçalho a mais custa 9 bytes (7.569 com 60 componentes). As análises já gravadas não
são refeitas: só uma nova análise usa a correção.

**Não verificável aqui:** o recall real só aparece numa nova rodada no `cloudrs`.

## Dependência de dois donos

**Problema.** `gpui` é declarado por `cloudrs` e por `cloudrs-ui`. Uma decisão que cita `gpui`
ligava aos dois, e como a aresta de dependência vinha antes das de símbolo e menção, bloqueava
a do dono que o texto de fato nomeia. Nos dados reais (4 decisões, todas com as duas arestas):
"icons" cita `cloudrs_ui::assets::Assets` (dono certo: `cloudrs-ui`), "sign-in" cita
`cloudrs --sign-in` (`cloudrs`), "logo" e "title bar" não citam nenhum dono.

**Regra.** Duas funções: `suggest_owned_dependencies` (dependência de **um** dono) segue no mesmo
ponto de antes, antes de símbolos e menções; `suggest_shared_dependencies` (**dois ou mais**
donos) roda depois de `suggest_mentions` e recebe os "donos citados" da decisão: donos com uma
linha **viva** (pendente ou confirmada; a menção rejeitada pela pessoa ou derrubada pelas regras
não conta), tipo `Affects`, cuja razão não é de dependência nem tem alarme de polaridade
(arquivo tocado, símbolo, menção, IA ou extrator).

- algum dono citado: nenhuma aresta de dependência compartilhada para a decisão (os citados já
  têm o vínculo deles; os outros donos não são evidência), qualquer que seja o número de donos;
- nenhum dono citado: uma aresta por dono, com a marca de compartilhada.

Os donos citados vêm de um índice por decisão (`DecisionTies`), montado uma vez por refresh, mais
as linhas que a própria decisão escreveu desde o início do laço: o custo deixou de ser
decisões x arestas do projeto.

**Convergência.** O resultado não depende da ordem em que a evidência chega. `revalidate`
invalida como `Rules` a aresta de dependência compartilhada, pendente ou confirmada pelas regras,
cuja decisão passou a ter algum dono citado por outra evidência (mesma função de donos citados
do derive). Não reabre sozinha: `blocked`/`reopens` só reabrem se a decisão mudou depois.
Na prática o índice único de arestas vivas (uma por tipo, origem e componente) já impede a
evidência nova de coexistir com a aresta de dependência do mesmo dono; a regra cobre o outro dono.
`MAX_OWNERS` é 2, então "três donos, dois citados" só existe no teste de unidade de
`cites_an_owner`.

**Formato da razão.** Quem lê a razão de dependência está só em `graph/derive.rs`, ao lado de
`dependency_reason`: `cited_dependency`, `cited_dependency_manifest` e `shared_dependency`. O
juiz (`triage_link`, `link_text`, `co_owners`) usa essas funções em vez de procurar a marca
`; também declarada por outro componente` no texto. O formato gravado não mudou.

**Juiz.** Para o vínculo de dependência compartilhada, `link_text` acrescenta `Shared dependency,
also declared by: <donos>` (os outros pendentes da mesma decisão e dependência, lidos da lista
que `gather` já tem; "another component" se não houver) e o `LINK_REVIEW_PROMPT` diz que os donos
competem: aceitar no máximo um, o que a decisão rege, **exceto** quando a decisão é sobre a
própria dependência (versão, features), e responder `human` quando o texto não diz qual. Sem a
exceção, "fixar gpui 0.2 no workspace" perderia um dono legítimo.

**Medições** (corpus, dois casos novos p09 "icons" e p10 "title bar", que reproduzem os reais):

| | precisão | cobertura | com o juiz | to_judge | refresh frio / quente |
| --- | --- | --- | --- | --- | --- |
| antes da regra, com os dois casos | 0,944 (34/36) | 0,895 | 1,000 | 29 | n/a |
| com a regra | 1,000 (34/34) | 0,895 (34/38) | 1,000 | 27 | 2,0 s / 0,95 s |
| antes (HEAD anterior, 51 decisões) | 1,000 (32/32) | 0,889 | 1,000 | 26 | 2,04 s / 1,00 s |

**Não coberto de propósito.** O caso "nenhum dono citado" (logo, title bar) não entra no corpus:
na contagem, aresta compartilhada conta como afirmada, então seria um erro conhecido; o
comportamento dele está no teste de unidade do texto do juiz. Arestas já gravadas e citadas por
outra evidência passam pela convergência acima; as demais não são reescritas.

## Riscos

- O refresh disparado pela tela do Mapa não aciona o juiz em segundo plano (a Revisão cobre).
- O descarte de "title bar → cloudrs" como "produto inteiro" vem de o prompt tratar o componente
  homônimo como o produto e pode se repetir; a exceção da dependência depende do juiz.
- **Pendente:** "session networking → architecture" é erro do proponente de vínculos da IA, não
  estrutural; exigiria um caso em `link_corpus.rs` com resposta de modelo em fixture gerada por
  modelo real (não trivial).
- O diagnóstico do extrator não conta itens malformados que o adaptador descarta; a correção
  ataca a causa provável, sem prova pela resposta bruta.
- Antes de `i18n::set` na partida (milissegundos), um job veria inglês.
