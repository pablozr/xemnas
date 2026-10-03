# Crítica de recuperação: rodada inicial e reteste

## Resultado

A primeira rodada não encontrou nada nas buscas feitas e tampouco uma regra para o caminho que eu havia chutado (`src/args.rs`, que nem existe neste clone). Na rodada atual, a busca sobre a inversão dos getters retornou uma decisão útil. `get_decision` pelo identificador curto que o MCP apresentou (`D:87ee609d`) recuperou escolha, motivo, status e ressalva de validação. Então houve uma melhora real: com o mapa atualizado, consegui reconstruir a orientação local sobre esses contadores sem recorrer a README genérico ou ler o código.

A resposta recuperada diz que a inversão deve ser preservada neste caso: em `Override`, glob sem `!` conta como whitelist e glob com `!` conta como ignore; não se deve trocar as delegações só para alinhar os nomes com `Gitignore`. A justificativa afirma que a captura e os artefatos sustentam essa leitura, mas também deixa explícito que o teste `counts` foi escrito e nenhum comando foi executado. Essa é uma decisão local do avaliador, não uma orientação dos mantenedores do ripgrep. Para alguém retomando o trabalho, esse aviso de proveniência e a limitação de validação são essenciais.

As demais perguntas seguem sem resposta recuperável. “mudança rejeitada tarefa recente” e “prazo de retenção de capturas aprovado” retornaram ausência de registro. Não vou inferir que nenhuma mudança foi rejeitada ou que inexiste prazo aprovado. A consulta sobre semântica de `!` trouxe a decisão dos contadores, que contém a regra geral útil no contexto, mas não esclarece por que uma alteração específica foi rejeitada. A consulta em inglês `polarity counters` não encontrou nada, enquanto “por que não corrigir contadores” e “Override num_whitelists” encontraram a decisão. Isso sugere sensibilidade à formulação/idioma.

## Problemas observados

A busca semântica teve um falso positivo evidente: perguntei por uma decisão inexistente sobre persistência da cor do terminal, e o MCP devolveu a decisão de `Override`. Isso enfraquece a confiança em consultas negativas. A busca não marcou a correspondência como fraca nem explicou por que considerou aquele resultado relacionado. O `get_decision` com referência inexistente, por sua vez, retornou erro claro, o que é melhor.

Houve divergência entre identificadores. O pedido desta avaliação forneceu UUID da decisão `01a1008e-6f72-7254-a436-2c6287ee609d` e do candidato `01a1008b-00c2-7225-a4e2-5915e059be10`, mas a busca expôs `D:87ee609d`. `get_decision` aceitou esse alias curto; rejeitou tanto `D:01a1008e-6f72-7254-a436-2c6287ee609d` quanto `D:01a1008e`. Portanto, para o MCP, “referência completa” parece significar o alias de oito caracteres que ele próprio retorna, não o UUID armazenado. Isso é utilizável após uma busca, mas torna difícil abrir diretamente um ID fornecido por outro subsistema. Prefixo `D:87ee` também falhou com “Referência ou consulta inválida”; não parece haver suporte a prefixos parciais.

`file_context` para `crates/ignore/src/overrides.rs` e para a variante com barras invertidas retornou “Nenhuma decisão ou regra ligada a esse arquivo no mapa do projeto.” O teste correto do caminho, portanto, não melhorou o acesso à decisão. O resultado não explica se o arquivo não está mapeado, se o vínculo de evidência não alimenta `file_context`, ou se há outra convenção de caminho. O fato de a busca textual achar a decisão mostra que ela existe, mas não que a consulta por arquivo funciona.

## Utilidade frente a docs/git

Nesta rodada, a ferramenta poupou uma busca manual e reuniu em uma resposta a decisão, a razão, a proveniência local e a ressalva sobre testes não executados. Para retomar um raciocínio de manutenção, isso tem valor concreto: eu teria aprendido por que o comportamento aparentemente invertido foi preservado e não teria “corrigido” os nomes por intuição. A resposta não substitui código, testes ou documentação dos mantenedores; ela recupera a justificativa registrada para este experimento. Se a pergunta é se a regra realmente corresponde ao ripgrep upstream, ainda é preciso conferir as fontes do projeto e executar os testes.

O benefício continua parcial. Não consegui recuperar qual mudança foi rejeitada na tarefa recente nem um prazo de retenção. A baixa precisão da busca negativa e o `file_context` sem ligação fazem a ferramenta exigir cautela. Eu agregaria: pontuação/explicação de relevância, uma resposta explícita de “nenhum resultado com confiança” separada de correspondências fracas, e indicação do status de cobertura do índice. Para referências, aceitaria o ID canônico informado e aliases curtos, exibindo a relação entre ambos. Para contexto por arquivo, mostraria quais caminhos foram indexados e permitiria abrir as decisões associadas a evidências anexadas.

## Estado das quatro perguntas

- **Por que os getters `num_ignores` e `num_whitelists` do `Override` parecem invertidos?** A decisão local diz que a polaridade de `Override` é deliberadamente invertida em relação a `Gitignore`: globs sem `!` contam como whitelist, globs com `!` como ignore; a delegação não deve ser trocada só para os nomes coincidirem. Trata-se do racional registrado, não validação independente do comportamento upstream.
- **Qual mudança foi rejeitada na tarefa recente?** Não recuperada nesta rodada.
- **Qual é a semântica de `!` em `--glob`?** A decisão encontrada afirma que `!` marca ignore e ausência de `!` marca whitelist para `Override` neste caso. A busca original sobre `--glob` não separou bem essa pergunta da dos contadores.
- **Existe prazo de retenção de capturas aprovado para este projeto?** Não recuperado nesta rodada. Resultado vazio não demonstra inexistência.

As requisições e respostas integrais de ambas as rodadas estão em `critic-retrieval.jsonl`.
