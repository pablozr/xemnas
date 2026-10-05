# Revisão por exceção e episódios de captura

Data: 04/10/2026. Status: slice implementado, com testes focados e revisão
independente. Não é aprovação normativa automática nem prova de produtividade.

## Resultado do experimento antes da implementação

O [piloto comparativo](resultado-piloto-recuperacao.md) executou três agentes novos
em fontes brutas, ADR e exportações reais offline do Xemnas. As seis tarefas por
condição tiveram encaminhamentos esperados na conferência do coordenador; não
houve pontuação cega independente ou tempo humano medido. Chamadas de ferramenta:
26/24/32, respectivamente. X não mostrou vantagem de passos nessa modalidade.
O experimento usou exportsnapshot, não injeção live; não prova produtividade de
desenvolvimento. Esse resultado motivou reduzir repetição e custo de consulta,
não ampliar automaticamente a autoridade do modelo.

## Implementado

- Natureza description/inference/normative/unknown na extração existente. Sem campo
  legado, Unknown. A mesma chamada de IA interpreta; não há um juiz obrigatório
  adicional por captura. Natureza não é autorização humana.
- Inferências são preservadas/rastreáveis, não apresentadas para confirmação como
  normas. Edição explícita pode restaurar Unknown para revisão.
- Descrição só é verificada por correspondência canônica exata e referência tipada
  a observação atual/elegível. Mencionar uma observação no artefato não basta.
  Normative/Inference nunca são sobrescritas por esse binding. Dirty/incerteza
  retiram o suporte atual; a proposta volta à revisão quando aplicável.
- Grupos estritos por Project, tipo, conteúdo, qualificadores e caminhos reduzem
  ocorrências equivalentes a uma oportunidade. Sem escopo/caminho conhecido, a
  identidade é conservadoramente restrita à captura. Negação, ressalva e alcance
  distintos não são fundidos por similaridade.
- Confirmação cria um alvo Decision/Claim e ledger lateral, sem aceitar todos os
  irmãos nem criar normas repetidas. Alvo revisado/encerrado invalida reaproveitamento.
  Registros e evidências individuais permanecem. Ator do journal é unknown quando
  não comprovado; ações de fixture não são intervenções humanas.
- Snapshot exibido é conferido tanto na confirmação quanto na adoção. A atomicidade
  completa de vínculos da adoção continua fora deste slice; falha parcial existente
  não é anunciada como sucesso integral.
- UI apresenta ocorrências paginadas em 20, seleção de fonte identificada e alvo
  principal da confirmação explícito. Polling usa elegibilidade da projeção, não
  o estado bruto Pending de um irmão já representado.
- Índice incremental dirty evita desserializar/reindexar todo o projeto em cada
  polling. SQL ainda percorre linhas elegíveis; custo não é constante.

## Expansão de captura

Proveniência imutável por captura conserva coordenadas disponíveis de adapter,
sessão/mensagem e relógios, sem reconstruir identidade humana pelo checkpoint ou
idempotency key. Episódios são descrições sobre artefatos capturados: não afirmam
estado atual do arquivo, teste aprovado, instalação ou autoridade normativa.

Diff usa o discriminante real `diff_hunk`. Contagem valida os intervalos dos hunks;
linhas legítimas com ++/-- são contadas. Snippet incompleto/malformado ou truncado
retorna desconhecido, não zero. Leitura bounded BLOB de 65.536 bytes considera o
tamanho original, inclusive NUL e fronteiras UTF‑8. Metadata é allowlisted/redigida.
Replay conserva a primeira proveniência; purge e upgrade mantêm isolamento.

Migrations novas: 30 grupos/ledger; 31 natureza/trace; 32 alvos tipados de resolução;
33 invalidação incremental; 34 episódios. Forward-only, legado sem autorização
retroativa. Wire Capture v1 permaneceu inalterado.

## Evidência final dirigida

Executados no estado final: Inbox 23, adoption 3, storage extraction 5, episódios
9 e architecture 13 testes passaram. Frontend 62 e main 7 passaram na validação
final da UI. Fmt global, check desktop e Clippy dos quatro crates all-targets
passaram. SAC teve bloqueios em etapas anteriores; relinks dirigidos foram usados
sem desligar política ou flags globais. Não anunciar uma única suíte completa verde.

Testes cobrem 800 ocorrências → uma oportunidade, 800 regras → um claim, mudança
de alvo, duas conexões, revisão stale, descrição dirty e binding normativo adversarial.
Reviews independentes fecharam os achados de autoridade, concorrência, ledger,
fontes UI, elegibilidade e diff/truncamento sem P1/P2 pendentes nos deltas examinados.
Não houve validação GUI desta nova apresentação de grupos.

Benchmark local de count+list, 30 amostras:

| Ocorrências | p50 | p95 |
| --- | --- | --- |
| 800 | 5,7 ms | 7,6 ms |
| 8000 | 57,9 ms | 65,7 ms |

Polling inalterado realizou zero updates de membership. A contagem de oportunidades
é redução potencial de repetição, não “800× produtividade”; tempo humano, perguntas
no uso real e custo monetário ainda precisam de estudo.

## Próxima prova

Repetir o protocolo com injeção live e tarefas de desenvolvimento equivalentes;
medir captura, manutenção, retomada, correções e esforço humano ativo. Validar a
classificação de natureza com provider real consentido em fixtures públicas, sem
inventar consentimento. Aprovação capturada na conversa só poderá substituir uma
confirmação se houver contrato verificável de autoridade, escopo e revisão.
