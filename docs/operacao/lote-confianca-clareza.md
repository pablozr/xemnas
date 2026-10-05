# Lote de confiança, clareza e medição

Data: 03/10/2026. Base: `d375fbc`.
Status: implementação integrada; builds debug/release e capturas executados.
Validação proporcional concluída com ressalvas de testes frontend/SAC e falha
preexistente de arquitetura. Não é certificação de todos os fluxos.

## Mudanças e motivos

- Preparação comum de conteúdo externo: redação de padrões reconhecidos, inclusive
  strings JSON decodificadas, antes dos cortes. Protege conteúdo introduzido pela
  edição e enviado por análise, sugestões, revisão e Visão. Não detecta todo segredo.
- Autorização vigente antes de chamadas e retries; ChatGPT revalida depois de obter
  token e acompanha rotação reconhecida pelo refresh. Revogação/troca de configuração
  bloqueia a próxima chamada; não se promete abortar HTTP iniciado.
- Prévia ampliada: documentos, conhecimento revisado e mapa deixam de ficar fora
  das categorias declaradas. O hash muda e requer nova autorização do usuário.
- Qualificadores de atribuição, alcance e validação acompanham conhecimento e
  versões. Extração exige trecho literal; edições preservam suporte inalterado e
  tratam texto novo como declaração humana. Ausência não significa autoridade global.
- Claims/sugestões derivadas conservam scope e versão-fonte. CAS impede confirmação
  de snapshot stale; pendentes stale podem ser recompostas sem apagar rejeições.
- Contexto admite unidades inteiras com suas ressalvas e registra omissões, inclusive
  quando nenhum item cabe. Pack, MCP, exportação e análise recebem essas restrições.
- Progresso por captura associa assessment à tentativa, com motivo/contagens
  persistidos atomicamente; retry não apresenta a falha antiga como resultado atual.
- Revisão e Diagnóstico mostram capturas recentes, fonte/modelo, estado e recuperação
  real. Polling visível de cinco segundos é limitado, sem requests sobrepostos;
  drafts e seleção são preservados, inclusive fora da janela paginada.
- Toast distingue regra/decisão; sugestões e filtros respeitam texto extenso;
  documentação explica captura/análise/revisão. Editores reutilizam Evidence.
- Dados malformados não são convertidos silenciosamente em informação ausente.

Migrations novas: 21 (qualifiers), 22 (assessment/tentativa), 23 (snapshot de scope
e versão das sugestões), 24 (versão-fonte da claim). Legado recebe arrays vazios ou
versão não informada; não se inventa informação histórica.

## Medição preparada

`crates/storage-sqlite/tests/context_corpus.rs` e sua fixture contêm 30 consultas,
dez famílias PT/EN/distrator, cinco positivas/cinco negativas e quatro famílias
reservadas. O relatório compara pack e entrega compacta e prepara 300 amostras de
latência, precisão/omissões e tokens estimados. Não altera a busca lexical.

Tokens são estimados por caracteres, não faturamento. Provider calls deste corpus
são zero; tempo humano ativo e intervenções humanas não são medidos pelo seed.
Baseline executada em debug otimizado: 30 consultas, 30 warmups e 300 amostras.
Latência de `build_pack`: p50 366 µs, p95 631 µs. Pack/compact300: precisão
17/26 (65,38%), recall 17/18 (94,44%); contaminação negativa 2/15 (13,33%).
São resultados sintéticos da busca atual, não melhoria comprovada ou meta cumprida.

## Evidência intermediária antes da retomada

Durante estados intermediários executaram com sucesso:

- application lib: 125 testes;
- ai-provider extractor: 21 testes e um ignorado por tocar keychain real;
- storage claims: nove testes; suggestions: dois;
- assessments: quatro; capture progress: dois; storage extraction: cinco;
- application extraction: 29;
- checks de formato e Clippy backend em etapas anteriores.

Esses resultados **não aprovam o diff final**, que recebeu correções posteriores.
No estado final, `cargo check --locked -p desktop-gpui` falhou ao carregar
`yoke_derive` por Controle de Aplicativo, erro 4551. Outros checks foram bloqueados
em `zerovec_derive`, build scripts ou `cargo-fmt`. Rustfmt direto e diff-check
executaram em arquivos alterados, mas não substituem compilação/testes finais.

Review independente backend e frontend foi realizado; achados foram corrigidos.
Os últimos deltas de fixtures de upgrade e seleção fora da página ainda precisam
de confirmação final. Nenhuma captura visual foi executada. Nenhuma política do
Windows, dado pessoal ou credencial foi alterada. Sem push.

## Critérios definidos antes da retomada

Em ambiente autorizado, executar fmt/check, Clippy e testes dos crates alterados
mais architecture; executar corpus normal/ignorado e publicar baseline com
denominadores; compilar desktop no Windows e validar demo nas paletas normal e
compacta, perguntando antes de captura que tome foco/clique.

Não foi implementada aprovação normativa automática, observador completo de
episódios, context router novo, busca híbrida ou instrumentação de faturamento.
Não anunciar redução de custo/atrito ou ganho de velocidade antes da comparação.

## Retomada após autorização de relink e capturas

O método de limpar somente artefatos Cargo bloqueados permitiu recompilar
`yoke-derive`, `zerovec-derive`, `profiling-procmacros`, `gpui_macros`,
`thiserror-impl` e `windows-interface` quando afetados. SAC permaneceu ligado.
Não houve mudança de código das dependências nem insistência no mesmo exe antigo.

- `cargo check --locked -p desktop-gpui`: passou; corrigidos namespace e docs
  públicas que só apareceram quando a compilação foi liberada.
- Clippy all-targets com `-D warnings`: passou para application, ai-provider,
  storage-sqlite e desktop-gpui.
- Backend: application lib 128 e testes de integração passaram; ai-provider lib
  10 e extractor 21 passaram, com um teste keychain intencionalmente ignorado.
- Storage: 134 testes passaram, com reexecuções dos targets bloqueados/corrigidos;
  corpus ignorado executou explicitamente. Não foi uma única bateria verde.
- Frontend: 59 testes passaram antes do último ajuste visual; testes finais
  compilaram, mas o exe permaneceu bloqueado por SAC após relink.
- Architecture: execução final com frontend teve 11/12; o check de cores acusa
  `ui/material.rs` e `ui/wallpaper.rs`. Confirmado em `d375fbc`, sem diff nesses
  arquivos: falha preexistente, não corrigida fora do escopo.
- `cargo fmt --all -- --check`: wrapper bloqueado. Rustfmt direto, edição 2021,
  verificou os módulos das quatro libs com sucesso; diff-check passou.
- Builds debug e release passaram. O release recém-gerado abriu em `--demo`
  pela captura offscreen, sem acessar dados pessoais.

Capturas autorizadas: Revisão, sugestões longas e grafo em Quiet normal e Carvão
compacto; Diagnóstico Quiet normal. Revisão final recapturada após tirar o histórico
de capturas do documento do candidato: qualificadores/evidência voltaram ao foco.
Arquivos temporários em `%TEMP%\xemnas\shots\post-eval-*.png`; não são assets do
produto. A inspeção visual não comprova todas as interações de teclado/retry.

Permanece pendente executar os testes finais frontend em ambiente permitido,
resolver separadamente a falha preexistente de arquitetura e medir humanos/custo
real. O commit `63f5cb4` existe no banco Git, mas não está no HEAD desta branch;
nenhum merge/cherry-pick foi feito automaticamente.
