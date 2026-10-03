# Decisão arquitetural de simulação: cache baseado no parser compartilhado
Status: aceita apenas pelo avaliador neste experimento; não é orientação dos mantenedores nem implementação upstream.

## Pergunta e escolha
Nesta simulação decidimos usar um cache de opções normalizadas em ignore, coordenado pela camada CLI/core. Esta escolha depende da nova decisão "ignore oferece um parser de opções compartilhado por todos os consumidores": sem o contrato único do parser, não existe uma chave de cache estável entre CLI e bibliotecas. O cache só faz sentido depois dessa decisão de parser compartilhado, que é sua dependência explícita.

## Alternativa rejeitada e razão
Rejeitamos caches independentes em CLI/core e ignore porque teriam chaves com semânticas diferentes e duplicariam invalidação. O contrato de cache será comum às chamadas de matching e à preparação de globs, garantindo opções equivalentes entre consumidores. Aceitamos a manutenção de uma política de invalidação única e o custo de sincronizar o ciclo de vida entre os componentes. Esta escolha é durável, afeta múltiplos componentes e condiciona evoluções futuras do contrato público.

## Evidência e alcance
Responsabilidades afetas: crates/core/flags/hiargs.rs, crates/ignore/overrides.rs e crates/ignore/walk.rs. Fixture sintética de relações; não houve mudança no código do clone por essa decisão.
