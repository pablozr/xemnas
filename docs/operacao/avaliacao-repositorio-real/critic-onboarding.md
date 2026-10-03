# Avaliação de onboarding — usuário crítico 1/5

## Escopo e como avaliei

Persona: desenvolvedor experiente, com pouco tempo e sem uso prévio do Xemnas. Para a etapa documental, li somente o `README.md` e `docs/operacao/operacao-e-referencia.md` do Xemnas. Para esta retomada, li `harness-notes.md`, consultei o harness autorizado no banco temporário de avaliação e revi os quatro PNGs em `ui-evidence/`.

Revisei screenshots existentes; não dirigi a GUI, não cliquei nem digitei nela. Portanto, as imagens mostram estados capturados pelo coordenador, não uma sequência de interação minha. As respostas do harness são JSON/Debug de casos de uso e banco reais; não são renderização nem prova do que a GUI mostra depois de uma captura. Não li relatórios de outros críticos nem código interno do Xemnas. A consulta de detalhe devolveu os artefatos da captura do ripgrep, que usei como evidência da decisão procurada.

Na primeira tentativa documental, executei `xemnas.exe --help`; não imprimiu ajuda e iniciou um processo sem título/janela visível, que encerrei. Como o startup não foi auditado, não afirmo que ele não acessou ou inicializou o diretório de dados padrão. Não repeti essa invocação.

## O que consegui entender antes de usar os dados reais

O README manda instalar Rust estável, compilar a aplicação e oferece uma demo em memória. A operação diz que a demo não inicia API, workers nem providers, então ela mostra a interface, mas não prova a coleta ou recuperação de uma decisão. O caminho de uso real passa por build local e ativação manual do plugin OpenCode (`npm ci`, wrapper local e reinício da sessão). Para avaliar um app desktop sem já estar comprometido com toolchains, esse custo vem antes do primeiro resultado.

A instalação do plugin é descrita com um caminho de exemplo amarrado à estrutura local de checkout do mantenedor. Eu precisaria adaptar o caminho e descobrir como confirmar que o OpenCode carregou o plugin e está ligado ao projeto certo. A documentação também alterna README em inglês com instruções/UI em português e contém uma limitação que diz que telas e navegação estão “entregues em paralelo”, embora o README anuncie vários desses fluxos como recursos. Isso deixou incerto qual build encontraria.

## O que a interface capturada comunica

- `01-review-empty.png`: Revisão mostra zero candidatos e informa que uma sessão OpenCode precisa registrar uma escolha de engenharia para o extrator propor algo. O texto explica por que está vazio, mas não oferece ali um passo explícito para registrar ou testar a primeira captura. “Captura ativa” no rodapé pode sugerir que está funcionando, sem dizer se há uma sessão conectada.
- `02-map-initial.png` e `03-graph-initial.png`: o projeto ripgrep já tem dez componentes mapeados, mas todos mostram zero decisões. Isso apresenta uma estrutura concreta do projeto, porém não equivale a memória arquitetural: o mapa sozinho não revela a escolha nem sua justificativa.
- `04-context-initial.png`: o envio de contexto ao agente está desligado, com um botão “Ativar”, e há “Testar uma tarefa” na navegação lateral. A tela explica fontes, seleção e limite de entrega; para primeira avaliação eu preferiria um teste com prévia que deixe claro o que será enviado antes de ativar.

As capturas parecem estado inicial com projeto já cadastrado, não onboarding limpo. Elas não mostram como cadastrar a pasta nem como passar de primeira sessão a candidato.

## Tentativa real com harness

O `projects` confirmou um projeto chamado `ripgrep` cadastrado para o clone temporário de avaliação. `inbox_filter` encontrou um candidato `pending` (`Decision`, significância 0,8, confiança 0,6). A fila, portanto, não estava vazia no banco consultado. `decisions` retornou zero decisões. Em `inbox_detail`, o candidato perguntava “Qual decisão durável a captura registra sobre rejects_alternative?” e sugeria “Manter a escolha sinalizada por: rejects_alternative, alternatives_compared, disagreement_uncertainty, maintenance_onboarding, unproven_assumption”. A justificativa declarava que isso foi inferido deterministicamente do envelope e que ainda requeria confirmação humana.

O detalhe trouxe dois artefatos: um texto do usuário com a leitura de `overrides.rs` e o pedido para registrar a decisão sobre a polaridade de `num_whitelists`/`num_ignores`; e uma resposta do assistente que articulava a decisão, sua razão (o `.invert()` no comportamento de overrides), alternativa rejeitada (trocar as delegações dos getters) e incerteza de validação. Há evidência suficiente na captura para um humano reconhecer a decisão. Porém, o resumo proposto pelo candidato não expressa a decisão: enumera os nomes dos sinais e usa uma formulação genérica. Isso desloca a síntese para a pessoa justamente na etapa em que o produto promete poupar a releitura da sessão.

`diagnostics` no banco isolado de avaliação informou 1 projeto, 1 captura, 2 artefatos, 1 candidato pendente e 1 job `analyze_capture` concluído, sem falhas. Isso dá um sinal útil de que a captura chegou e foi processada. Também consultei diagnósticos em um banco próprio vazio em `onboarding-data`; ele mostrou zero projetos/capturas/candidatos. A primeira chamada de diagnostics foi feita no `ui-data` temporário autorizado; não toquei no banco/runtime do desktop. A chamada posterior no DB próprio cumpriu a orientação de isolar essa consulta.

A confirmação não foi executada. O harness identifica sua própria confirmação como automatizada e não equivalente à aprovação humana; além disso, a orientação para esta etapa foi consulta somente de leitura. Portanto, a decisão foi descoberta na evidência da captura, mas não preservada como uma decisão aceita: a listagem de decisões continua vazia. Os resultados de Debug não me permitem dizer como um humano revisaria/confirmaria esse candidato na GUI; não há screenshot de candidato preenchido entre as evidências fornecidas.

## Valor em relação a ler docs e Git

A documentação geral e o histórico Git podem mostrar regras do projeto, mas não localizam por si só a decisão feita numa sessão nem a justificativa em primeira pessoa. Aqui o Xemnas juntou a pergunta, evidências de usuário/assistente e o estado do job em uma consulta, o que é um ganho de descoberta. A própria captura, porém, já continha uma formulação explícita da decisão, enquanto o candidato criado a partir dela ficou genérico. Com o detalhe em texto corrido e um artefato grande, a economia real em relação a abrir a transcrição e fazer busca textual é pequena até o produto resumir melhor e levar a uma decisão confirmada.

O primeiro benefício real que consegui demonstrar foi localizar a sessão e apontar onde está a evidência da decisão. Não consegui completar “preservar”: ela ainda não aparece em `decisions`. O fluxo também depende de integração OpenCode e da pessoa confirmar a interpretação; a automação não elimina essa revisão.

## O que me faria desistir e o que agregaria valor

Eu desistiria se depois de instalar Rust, compilar, configurar o plugin e reiniciar o OpenCode, a Revisão continuasse vazia sem explicar se faltou captura, processamento ou decisão extraível. As telas capturadas explicam o estado vazio, mas não guiam a configuração; o diagnóstico consultado tem bons contadores, mas é informação técnica que um usuário precisa descobrir onde abrir.

Para agregar valor de verdade, eu esperaria que o candidato dissesse claramente: “preservar a inversão da polaridade dos contadores em `Override`, porque `.invert()` troca a semântica de whitelist/ignore”, citasse os dois artefatos/trechos relevantes, e permitisse confirmar essa decisão com um clique e depois encontrá-la em Decisões. Um onboarding valioso provaria isso usando uma sessão real e mostraria conexão, captura, processamento, revisão e resultado, sem exigir que o usuário inferisse sozinho o estado entre telas.

## Evidência consultada

- Documentação inicial: `README.md` (“Getting started”, “How it works”, “Integrations”) e `docs/operacao/operacao-e-referencia.md` (“Instalação e execução”, “Integração com o OpenCode”, “Limitações conhecidas”).
- UI capturada pelo coordenador: `ui-evidence/01-review-empty.png`, `02-map-initial.png`, `03-graph-initial.png`, `04-context-initial.png`.
- Harness e DB temporário: `projects` encontrou `ripgrep`; `inbox_filter` encontrou um candidato pendente; `inbox_detail` trouxe dois artefatos; `decisions` veio vazio; diagnostics registrou captura/job concluído no DB de avaliação e estado vazio no DB isolado próprio.
- Limite: nenhuma ação de confirmação foi feita; nenhum screenshot mostra a revisão dessa captura na GUI.
