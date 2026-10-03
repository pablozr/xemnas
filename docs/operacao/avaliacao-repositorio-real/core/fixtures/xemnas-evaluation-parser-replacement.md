# Decisão arquitetural de simulação: substituir a fronteira do parser
Status: aceita apenas pelo avaliador neste experimento; não é uma decisão dos mantenedores nem mudança implementada.

## Pergunta e escolha
Nesta simulação decidimos substituir a decisão local anterior de manter parsing exclusivamente em CLI/core. A nova escolha é que ignore ofereça um parser de opções compartilhado por todos os consumidores, enquanto CLI/core só traduza a interface de terminal para esse contrato público. Esta decisão substitui explicitamente a escolha anterior "Manter matching independente do parsing CLI" neste cenário local.

## Alternativa rejeitada e razão
Rejeitamos manter parsers diferentes em cada consumidor: isso duplicaria a interpretação de flags e faria consumidores CLI e biblioteca divergirem. A vantagem esperada é um contrato único de opções; aceitamos o custo de acoplar ignore ao modelo de opções. O tradeoff altera a fronteira entre CLI/core e ignore, afeta o contrato público e condiciona mudanças futuras por pelo menos doze meses.

## Evidência e alcance
Responsabilidades afetas: crates/core/flags/hiargs.rs, crates/ignore/overrides.rs e crates/ignore/walk.rs. Este texto registra apenas uma escolha sintética de teste e não descreve código já alterado.
