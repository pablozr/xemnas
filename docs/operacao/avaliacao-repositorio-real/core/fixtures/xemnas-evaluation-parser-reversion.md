# Decisão arquitetural de simulação: preservar independência do parser
Status: aceita apenas pelo avaliador neste experimento. Não é decisão de mantenedores nem mudança implementada.

## Escolha aceita e substituição explícita
Nesta nova simulação decidimos revogar a escolha local mais recente de oferecer um parser compartilhado em ignore. O parser volta a pertencer exclusivamente à camada CLI/core; ignore recebe opções normalizadas e permanece independente do parsing dos argumentos CLI. Esta decisão substitui explicitamente a decisão local "O componente ignore deve oferecer um parser compartilhado de opções para todos os consumidores". O objetivo é recuperar a reutilização de ignore sem impor o modelo de flags aos consumidores de biblioteca.

## Alternativa rejeitada, tradeoff e futuro
Rejeitamos manter o parser compartilhado em ignore porque acopla o contrato público da biblioteca à interface de terminal. Aceitamos que CLI/core mantenha a tradução e normalização, mesmo ao custo de adaptadores por consumidor. A nova fronteira afeta múltiplos componentes, condiciona evoluções futuras e será preservada por pelo menos doze meses. É uma escolha aceita e durável deste cenário sintético, não apenas uma ideia ou alteração pontual.

## Evidência e alcance
Responsabilidades: crates/core/flags/hiargs.rs, crates/ignore/overrides.rs e crates/ignore/walk.rs. Nenhum código foi alterado por este documento; é exclusivamente fixture local para testar rejeição de proposta de relação.
