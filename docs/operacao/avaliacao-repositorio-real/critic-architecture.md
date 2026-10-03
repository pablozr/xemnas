# Avaliação crítica: mapa e grafo do ripgrep

## Contexto da avaliação

Comparei as capturas reais fornecidas pelo coordenador (ui-evidence/02-map-initial.png e ui-evidence/03-graph-initial.png) com as quatro respostas que eu havia obtido no controle lendo o clone ripgrep. Não operei a interface; a avaliação visual se limita às imagens. Consultei o harness read-only para map_entities, map_edges e context_pack, usando o projeto 01a10082-807b-7381-b997-c1457b10a1e3. As três chamadas retornaram status=ok, duration_ms=0; não alterei o banco.

## O que o mapa e o grafo realmente acrescentaram

O mapa confirmou que a descoberta reconheceu dez diretórios Cargo como componentes: globset, grep, grep-cli, grep-index, grep-matcher, grep-pcre2, grep-printer, grep-regex, grep-searcher e ignore. Isso ajuda como índice inicial do repositório. Os cartões mostram padrões de caminho e zero decisões. map_entities confirmou descrições e aliases vazios. Não há, portanto, informação de função ou responsabilidade por componente.

A captura do grafo mostra os mesmos componentes sem conexões. map_edges confirmou edges: [] e suggested: []; os dez nós têm profundidade zero. As posições visuais não explicam relação alguma. O cabeçalho da própria tela diz que a arquitetura é “desenhada pelas decisões confirmadas”, e o grafo sem relações torna esse limite visível: neste estado não é um mapa de arquitetura, é um conjunto de pontos sem arestas.

Para as perguntas do controle, o incremento foi praticamente zero:

1. **Caminho de --glob:** o mapa identificou grep-cli e ignore como lugares candidatos, mas não mostrou o fluxo entre eles, nem o papel de crates/core. O código do clone respondeu a cadeia concreta LowArgs → hiargs::globs → OverrideBuilder → WalkBuilder; a tela não ajudaria a recuperar esses passos.
2. **Matcher/searcher/printer/core:** nomes grep-matcher, grep-searcher e grep-printer aparecem como cartões, mas não há direção, contrato ou papel. core, que coordena os três no código, nem foi descoberto entre os dez componentes. O grafo não corrige essa lacuna.
3. **Onde mudar prioridade:** os nomes sugerem diretórios para explorar, mas não localizam a lógica de precedência (a implementação compartilhada de matching em ignore, o wrapper de overrides e a composição das flags em core). O clone e buscas por símbolos foram necessários.
4. **Positivo versus !:** nenhuma informação do mapa ou do grafo expõe a semântica. A resposta veio dos comentários, documentação da travessia e ordem de matching no código.

O context_pack para a tarefa formulada retornou pacote vazio: claims: [], decisions: [], used_chars: 0, apesar de status=ok. Isso é coerente com as capturas exibindo zero decisões. Para esta tarefa técnica, não trouxe código indexado nem uma trilha de evidência. O sucesso da chamada significa apenas que o caso de uso retornou o pacote vazio; não significa que o pedido foi respondido.

## Opinião franca

Hoje eu abriria o mapa uma vez para ver se havia alguma estrutura e voltaria ao editor/repo. Para entender a arquitetura, essas telas custaram uma interação e me deram nomes que eu já obtinha de Cargo, sem reduzir a busca no código. O grafo, em particular, promete uma resposta relacional e entrega nós soltos. Sua distribuição visual pode parecer organização, mas eu não a interpretaria como arquitetura.

A descoberta automática é um começo útil como inventário de diretórios, desde que seja rotulada como tal. Ela também merece revisão: a separação grep-cli/grep perde o crate core, central para as perguntas; os cartões não informam isso. Um nome de crate não substitui uma descrição de responsabilidade.

Eu não manteria um grafo manual como duplicata de imports, dependências Cargo e chamadas já navegáveis no editor. O que teria valor incremental seria um mapa pequeno e consultável de **responsabilidades e decisões que o código sozinho não explica**, ligado a fontes verificáveis: por exemplo, “o CLI transforma argumentos em overrides; ignore define precedence; core coordena busca” com referências a arquivos e símbolos. Arestas precisam dizer o tipo de relação e ter evidência; sem isso, a visualização não deve sugerir causalidade ou fluxo. Se forem relações mecânicas deriváveis de Cargo/imports, prefiro derivá-las automaticamente e deixar a documentação explicar apenas as razões e exceções.

Para esta pergunta, eu preferiria uma busca no repo ou um pacote de contexto que cite diretamente hiargs.rs, overrides.rs, walk.rs e core/search.rs a abrir o mapa. O context pack atual é vazio, então ainda não oferece esse atalho. Uma tela de bloco poderia valer como navegação para uma página rica, mas não foi demonstrada por estas evidências; não conto isso como benefício observado.

## O que está ruim e o que priorizaria

- **Problema principal:** as telas chamam de mapa/arquitetura um inventário de diretórios sem responsabilidades nem relações. Deixar isso explícito evitaria interpretação errada.
- **Descoberta incompleta para o objetivo:** não aparece core, apesar de ele ser essencial no fluxo estudado. Eu ajustaria a descoberta ou ofereceria revisão/edição das fronteiras de componente.
- **Grafo sem informação:** nós espalhados, sem arestas e com todos em profundidade zero não explicam nada. Quando não houver arestas, mostre um estado vazio que explique como relações são construídas e de onde vêm, em vez de insinuar uma topologia.
- **Sem trilha de evidência visível:** nomes e padrões de caminho não mostram por que um componente importa. Vincular responsabilidades e relações a arquivo/símbolo/trecho verificável agregaria valor.
- **Pacote de contexto sem cobertura de código:** context_pack respondeu vazio para uma pergunta cuja fonte existe no projeto. Integrar indexação/retrieval de documentos ou código, e distinguir “não há conhecimento cadastrado” de “nenhuma evidência encontrada”, seria prioridade maior que ornamentar o grafo.
- **Não transformar em manutenção duplicada:** importar estrutura de Cargo automaticamente; pedir registro humano apenas para decisões, exceções e contratos que a estrutura não contém.

Minha ordem de prioridade seria: (1) retrieval com citações úteis e cobertura do código/documentação; (2) componente com responsabilidade concreta e fonte; (3) relações tipadas e justificadas, com mecanismo barato de atualização ou derivação; (4) refinamento visual do grafo. Sem os três primeiros, a visualização não economiza meu trabalho.

## Evidências e limites

- Captura de blocos: ui-evidence/02-map-initial.png mostra dez cartões com paths Cargo, descrições ausentes na inspeção do harness e zero decisões.
- Captura de grafo: ui-evidence/03-graph-initial.png mostra dez nós isolados e contadores zero; harness confirmou dez entidades, zero arestas e zero sugestões.
- context_pack com tarefa explícita sobre as quatro perguntas: pacote retornado sem claims/decisions, used_chars=0.
- Não testei interação, busca, abrir cartões, sugestões, persistência, ou comportamento de outras telas. Não concluo nada sobre a qualidade dessas funções.
- O controle de código está documentado em critic-architecture-control.md; esta avaliação compara somente o que foi realmente mostrado com as respostas daquele controle.
