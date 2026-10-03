# Retomada com release do checkout

**Data:** 03/10/2026. **Status:** retomada concluída; núcleo e fluxos GUI auditados.

## Proveniência e execução

O release deste checkout foi recompilado normalmente com `cargo build --locked --release -p desktop-gpui --bin xemnas` (52,21 s). O executável com SHA-256 `D12B26B59D1C73039A036F2586E2EE68AACAD1F5BB185573B9C5B0E6FFD8EEBB` abriu e foi usado nos testes abaixo. Nenhuma política do Windows foi alterada. A explicação de cache por hash foi trazida pelo usuário; esta execução comprova a abertura do novo arquivo, sem isolar a causa interna do SAC. O commit 63f5cb4 relatado pelo usuário não estava neste checkout.

Uma remoção prévia de artefatos do harness foi rejeitada pela revisão automática com apenas “blocked by policy”; nada foi removido por essa chamada. Uma compilação debug normal posterior do harness, após ampliar sua instrumentação, passou e o novo executável rodou. Não se repetiu a remoção por outra ferramenta.

## Documento, candidatos e adoção

A fixture [fronteira CLI/matching](fixture-arquitetural-pendente.md) foi indexada como terceiro documento. É um ADR sintético aceito somente pelo avaliador, não posição dos mantenedores nem alteração upstream. Atualizar a Visão também acionou `Documents::propose` e análise real com Luna medium pelo bridge de avaliação. A extração produziu uma decisão (significância 0,8) e uma regra (0,7), ambas com confiança 1 e critérios arquiteturais. Isso demonstra que o provider extrai um caso durável explícito; a ausência anterior de candidato numa rotina local não era prova de falha.

A interface mostrou evidência completa e a prévia de adoção: `ignore` disponível e `core`/`docs` sem componente. O coordenador confirmou regra e decisão com o vínculo de `ignore`; persistiram uma decisão, uma claim e as respectivas arestas. A tentativa de inserir ressalva pelo helper de digitação não alterou visualmente os campos e foi cancelada: não afirmar que essa ressalva foi salva. As escolhas resumidas omitiram o qualificador “simulação local”, embora a evidência o mostrasse. Confirmar pela automação representa avaliação, não aprovação dos mantenedores.

## Sugestões e mapa

A decisão gerou duas sugestões de regras. Uma foi confirmada e a outra rejeitada; o banco registrou `confirmed`/`rejected` e duas regras vigentes no total. Criar `core` pela sugestão elevou o mapa de dez para onze entidades e gerou duas arestas propostas. O vínculo da decisão foi confirmado e o da regra rejeitado; persistiram `confirmed_at` e `invalidated_at`, respectivamente. O detalhe de `core` mostrou decisão, vizinhança e linha do tempo. A sugestão de transformar `docs` em componente foi deixada pendente: um diretório documental não prova uma responsabilidade arquitetural.

## Falha e reprocessamento

Somente o bridge do experimento foi interrompido. Uma captura sintética válida, claramente marcada como local, entrou pela API real (HTTP 201); o worker gerou `analyze_capture` Failed com uma tentativa. A tela mostrou “Reprocessar”. Após restaurar o bridge, o coordenador clicou nessa ação: o mesmo job completou com duas tentativas, gerou um candidato e esse candidato foi rejeitado na GUI. O banco confirmou `dismissed`. Não foram forjados estados por SQL nem houve duplicação do job.

## Problemas observados

- Confirmar a regra mostrou “Decisão criada e ligada ao mapa”, embora o caso tivesse criado uma claim e nenhuma decisão naquele instante.
- Na lista de sugestões, frases longas ultrapassaram a largura e invadiram os botões de confirmação/rejeição, na janela de 1.442 × 1.025 px.
- O diagnóstico continuou mostrando “Executando” após o job já estar completo no banco; a Revisão precisou de Atualizar para mostrar o candidato. É atraso de atualização observado, sem perda de dados demonstrada.
- Documentação diz que não vira decisão, mas a geração de Visão acionou proposta/análise documental e produziu candidatos. O texto da tela não explica esse caminho.
- Uma regra resumida e uma regra derivada parcialmente sobrepostas ficaram vigentes. Isso exige revisão de redundância; duas regras não significam duas informações novas.

## Evidências

[Snapshot final da GUI](../logs/resumed-state-final.json), [captura sintética de falha](../logs/failure-capture-input.json) e [recibo](../logs/failure-capture-receipt.json). O snapshot contém somente registros do experimento público, sem banco, token, discovery ou perfil privado. As imagens selecionadas estão em `../evidencias/37-job-failed.jpg`, `40-map-suggestions-overflow.jpg`, `41-map-links-reviewed.jpg` e `42-core-component-linked.jpg`.

## Núcleo instrumentado e encerramento provisório

Os [retestes do executor](nucleo-instrumentado.md) e as respostas em [resumed-core-tests.jsonl](../logs/resumed-core-tests.jsonl) cobrem exportação, revisão semântica, entidades e remoção. O coordenador auditou independentemente o snapshot purgado: registros de projeto, decisões, candidatos, claims, entidades, arestas, sugestões e capturas ficaram zerados; dois jobs completed persistiram; `integrity_check=ok` e FK sem erro. Ver [auditoria](../logs/resumed-core-audit.json).

Em outro snapshot, um candidato artificial pelo FakeCandidateExtractor foi confirmado pela API real, ligado a ignore e usado em `conflicts_with`/`supersedes`; a decisão anterior ficou superseded. Isso valida operações de relação, não qualidade de extração/geração pelo provider. [Auditoria do coordenador](../logs/relations-core-audit.json) e [entrada/saída](../logs/relations-core-tests.jsonl).

A última compilação do harness acrescentou uma porta pública para criar uma proposta artificial de relação, mas o executável recebeu WinError 4551 antes de rodar. Nenhuma proposta foi inserida; confirm/reject de relação não foram executados. O teste de integração específico de SQLite foi lido: usa Judge sintético e verifica espera por confirmação e rejeição sem retorno. Compilou em 17,08 s, porém seu binário também foi bloqueado antes de executar. [Registro da tentativa](../logs/relation-suggestions-validation.json). Não contar testes verdes que não rodaram.

Para completar o caminho real, duas novas fixtures documentais de parser/cache foram preparadas no clone isolado. O release foi iniciado novamente, mas o uso da tela foi interrompido pela tecla Escape antes de navegar/analisar essas fixtures. Não afirmar que geraram candidatos ou sugestões. A retomada de automação de tela ficou dependente da resposta do usuário; nenhum clique posterior ao Escape foi executado.

## Conclusão posterior

O usuário autorizou retomar a tela. O [teste final de relações pela GUI](relacoes-gui-final.md) gerou propostas com provider real e exercitou confirmar/rejeitar, com prova persistente. Isso resolve a pendência de runtime descrita acima; o bloqueio dos binários auxiliares não foi convertido em resultado verde.
