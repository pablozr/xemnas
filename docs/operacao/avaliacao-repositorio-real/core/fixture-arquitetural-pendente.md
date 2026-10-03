# ADR de simulação local: fronteira entre CLI e matching

Status: aceita somente pelo avaliador para este experimento. Este arquivo é uma fixture de avaliação e não uma decisão dos mantenedores do ripgrep. Não descreve uma mudança implementada no upstream.

## Decisão arquitetural do cenário

Manter a responsabilidade de construir globs e traduzir opções na camada CLI/core, e manter matching e precedência no componente ignore. A aplicação consumidora coordena esses componentes; ignore não deve depender da camada de argumentos do CLI.

## Alternativa rejeitada e motivo

Rejeitamos mover parsing de argumentos para ignore: isso acoplaria uma biblioteca reutilizável à interface de um executável e prejudicaria outros consumidores. Preservar esta fronteira permite evoluir a interface CLI sem exigir que todos os consumidores da biblioteca conheçam flags.

## Regras duráveis deste cenário

Toda alteração de parsing de argumentos fica na camada CLI/core. O componente ignore deve permanecer independente do parsing CLI. A fronteira se aplica a futuras alterações de matching, não apenas a uma função.

## Fontes e limites

Arquivos candidatos ao vínculo: crates/core/flags/hiargs.rs, crates/ignore/src/overrides.rs e crates/ignore/src/walk.rs. O avaliador observou a composição desses módulos, mas não executou os testes do clone. A fixture testa extração, revisão e vínculo; não autoriza modificar código upstream.
