# Ideias de virada: da memória que lembra à decisão que guarda

**Data:** 06/10/2026.
**Pergunta:** quando lembrar entre sessões já vem de graça nos agentes, o que só o Xemnas
consegue fazer e pode fazer o produto deslanchar?
**Status:** Aberta. Pesquisa e proposta; nada implementado, demanda não validada. O usuário
marcou a ideia 1 (language server) para explorar a seguir (06/10/2026).

## O mercado mudou o terreno

- **Memória virou recurso nativo.** O Claude Code tem memória automática e, desde maio de 2026,
  uma consolidação entre sessões (Auto Dream); a Cognition lançou o *Dreaming* e o padrão Agent
  Memory Repo em 05/10 ([nota do Sonho](sonho-consolidacao-da-memoria.md)); servidores de memória
  compartilhada entre agentes já existem
  ([XDA](https://www.xda-developers.com/gave-claude-code-codex-cursor-same-memory/),
  [The New Stack](https://thenewstack.io/ai-coding-tool-stack/)). Vender "o agente lembra" é
  competir com o padrão de fábrica.
- **Agentes já obedecem a diagnósticos.** Quase todo agente de código fala LSP e lê os erros
  depois de cada edição, corrigindo no mesmo turno
  ([Dubovoi](https://olegdubovoi.com/publications/lsp-for-ai-coding-agents-the-protocol-your-agent-isnt-using-yet/),
  [OpenCode LSP](https://opencode.ai/docs/lsp/)). Esse é o canal de realimentação mais
  respeitado pelos agentes, e ninguém o usa para decisões de projeto.
- **Agentes em paralelo brigam.** Em 33.596 PRs de agentes, 79,4% disputavam o mesmo alvo ao
  mesmo tempo; o conflito entre agentes diferentes chegou a 41,7%, e 26,8% dos conflitos eram
  de modificar/apagar, ou seja, divergência sobre o que deve existir
  ([resumo](https://codex.danielvaughan.com/2026/07/28/agent-pr-merge-conflicts-concurrent-coding-agents-codex-cli-worktree-isolation-coordination-defence/),
  arXiv:2607.04697, confirmar). Admitir a escrita só depois de declarar a intenção subiu o
  sucesso de 23,3% para 50%, mas serializou quase tudo ([Claim Plane](https://arxiv.org/abs/2608.00947)).
- **Open source está sob cerco.** Mantenedores fecham PRs externos em massa (Ghostty, tldraw) e o
  GitHub criou limites de envio; o esforço que filtrava contribuições ruins sumiu
  ([OSFY](https://www.opensourceforu.com/2026/02/github-weighs-pull-request-kill-switch-as-ai-slop-floods-open-source/),
  [Forkable](https://www.forkable.io/p/when-ai-floods-the-commons-maintainers)).

**Consequência:** o que o Xemnas tem e uma memória automática não tem é **autoridade**:
decisões revisadas por uma pessoa, ligadas ao código e com fonte. A virada é parar de só
*lembrar* o agente antes da tarefa e passar a **guardar a decisão no momento em que o código
muda**, em qualquer agente e qualquer editor, inclusive de quem não usa o Xemnas.

## As quatro ideias

Elas não repetem as propostas anteriores ([mercado e ideias](mercado-e-ideias.md),
[backlog](backlog-de-ideias.md)):
reaproveitam os verificadores de regra listados no [backlog](backlog-de-ideias.md#regras-e-verificação)
(ainda não implementados) e mudam *onde* e *para quem* eles agem.

### 1. Decisões como diagnósticos: o Xemnas vira um language server

**Cena.** O agente adiciona `openssl` ao `Cargo.toml`. No mesmo turno, junto com os erros do
compilador, recebe: `xemnas: viola D:tls-rustls — usar rustls, não openssl (motivo: build
estático no Windows)`. Ele corrige sozinho. A pessoa vê o mesmo sublinhado no editor, com a
decisão no *hover* e duas ações: abrir a decisão ou registrar uma exceção.

**Como.** Um binário `xemnas-lsp` que lê o banco local em modo somente leitura e publica
diagnósticos só para regras com verificador (`forbid_added`, `forbid_dependency`,
`forbid_edit`). O resto das decisões que valem para o arquivo aparece como *hover* e
*code lens*, nunca como erro. Um servidor serve VS Code, Zed, Neovim, JetBrains, OpenCode e
qualquer agente com LSP.

**Por que é diferente.** A injeção age antes da tarefa e depende de o agente ler; o
diagnóstico age **depois de cada edição**, no canal que o agente já trata como erro a corrigir.
A medição L2 (adesão) passa de auditoria posterior a prevenção.

**Custo.** Sem IA e sem rede: casamento de padrão no arquivo salvo, com índice das regras por
caminho carregado uma vez. Orçamento proposto: menos de 5 ms por arquivo e memória de poucos MB
(medir com `XEMNAS_PERF=1`).

**Risco.** Falso positivo vira ruído que o agente "conserta" errado. Por isso, só regra
confirmada e com verificador vira diagnóstico, com severidade `warning` por padrão.

### 2. Repetições que viram regra: o custo de se repetir, contado

**Cena.** "Você disse ao agente *não use `unwrap` em `application`* em 4 sessões nesta semana.
Virar regra?" Um clique, e a regra entra na injeção e, se couber, no verificador da ideia 1.

**Como.** As capturas já trazem `user_text` por sessão. Agrupar instruções corretivas da pessoa
(negação mais termo de código: "não", "sem", "nunca", "use X em vez de Y") pelos mesmos termos
de busca e ponte PT/EN da seleção de contexto. Duas ocorrências em sessões diferentes geram uma
proposta, com as falas como evidência. É a fonte de regra com maior sinal que existe: a pessoa
já decidiu, e repetiu.

**Por que é diferente.** As memórias automáticas gravam o que o agente acha importante; esta
grava o que **custou paciência**, e mostra o número. "Você não precisou repetir nada há 12 dias"
é a métrica de valor que a pessoa sente, e a frase que ela compartilha.

**Custo.** Léxico local; IA só para redigir a regra quando a pessoa aceita a proposta.

### 3. Barramento de decisões entre agentes paralelos

**Cena.** Três sessões em worktrees diferentes do mesmo projeto. Na sessão A, a pessoa decide
"o cache fica em `storage`, não em `application`". No próximo turno, B e C recebem pelo gancho
de injeção: `decisão provisória da sessão A, há 4 min: …`. Quando B começa a editar o
componente que A está mudando, recebe um aviso: `a sessão A está alterando "storage-sqlite"
(intenção: mover o cache)`.

**Como.** Duas peças sobre o que já existe:
- **Decisões provisórias**: candidatos ainda não revisados de uma sessão irmã entram na injeção
  das outras, marcados como provisórios e com a sessão de origem; expiram se forem descartados.
- **Intenções por componente**: o mapa já liga arquivos a componentes. Cada sessão ativa
  publica os componentes que tocou nos últimos minutos (dos `diff_hunk` capturados); o gancho
  avisa quando outra sessão entra no mesmo componente. É um aviso, não uma trava: a lição do
  Claim Plane é que travar serializa tudo.

**Por que é diferente.** Worktrees isolam o texto, não as escolhas. Ninguém coordena a
*divergência de decisão* entre agentes, que é o conflito de modificar/apagar. E o usuário
já trabalha assim (Orca com várias worktrees): o dogfood existe hoje.

**Custo.** Uma consulta indexada por turno no gancho; nenhuma chamada de IA nova.

### 4. O contrato do repositório: decisões que valem para agentes de fora

**Cena.** Um mantenedor de projeto open source confirma 20 decisões no Xemnas e publica
`.xemnas/` no repositório. Quem clona e trabalha com qualquer agente recebe essas decisões pelo
`AGENTS.md` gerado, pelo MCP ou pelo language server da ideia 1. No CI, uma Action roda os
mesmos verificadores e **só comenta quando o PR viola uma decisão confirmada**, com o link e o
motivo; caso contrário, fica calada.

**Como.** Exportação já proposta no formato Agent Memory Repo (nota do Sonho), mais os
verificadores em formato declarativo e um executor de linha de comando (`xemnas check`) que
roda sem o app nem banco.

**Por que é diferente e por que pode explodir.** É a resposta para a crise de PRs gerados por
IA que não exige banir ninguém: o agente do colaborador recebe as regras da casa antes de
escrever, e o mantenedor deixa de explicar a mesma coisa em cada PR. É também o único laço de
distribuição natural do produto: cada repositório que adota mostra o Xemnas a todos que
contribuem nele. Diferente de um revisor de IA (CodeRabbit, Qodo): sem IA no CI, sem
comentário de estilo, só decisões que uma pessoa confirmou.

**Risco.** Exige publicar decisões, o que hoje é local por princípio; o mantenedor escolhe o
que sai, com prévia do arquivo. Formato público vira contrato: precisa de ADR antes.

## Sequência proposta

1. **Repetições que viram regra** (2): só backend, usa capturas existentes, dá o número de valor
   e alimenta todas as outras com regras melhores.
2. **Verificadores** (base da 1 e da 4): os cinco tipos já desenhados, com portão de
   assertividade (precisão sobre diffs reais rotulados, falsos positivos ≈ 0) e de desempenho.
3. **Language server** (1): o mesmo motor, entregue ao editor e ao agente.
4. **Barramento entre agentes** (3): dogfood imediato nas worktrees do próprio Xemnas.
5. **Contrato do repositório** (4): depois de o formato se provar em uso, com ADR e piloto em
   um repositório público do próprio usuário.

## Como saber se valeu

| Ideia | Sinal de que funciona | Sinal para abandonar |
| --- | --- | --- |
| 1 | Violações corrigidas pelo agente no mesmo turno ÷ violações emitidas ≥ 0,8 | Agente "conserta" errado ou a pessoa silencia o servidor |
| 2 | Instruções repetidas por semana caem depois que viram regra | Propostas aceitas < 50% |
| 3 | Menos candidatos contraditórios entre sessões irmãs; conflitos de merge no dogfood | Avisos ignorados em > 80% dos casos |
| 4 | PRs externos que respeitam decisões sem comentário do mantenedor | Mantenedor desliga a Action no primeiro mês |

Nenhum desses números foi medido. O protocolo de corpus às cegas e holdout selado da seleção de
contexto vale para os verificadores (1, 2 e 4); o barramento (3) se mede no uso real.

## Segunda rodada (06/10/2026)

O usuário pediu ideias que ainda não existem neste contexto. Escolhidas para aprofundar:
[Procurador](procurador.md) e [mapa de cegueira](mapa-de-cegueira.md). As outras ficam
anotadas:

- **Taxa de arrependimento**: ao adotar uma biblioteca, mostrar quantos projetos parecidos a
  removeram em um ano, para onde foram e por quê, a partir de um índice local pré-calculado do
  histórico público de manifestos. A mineração de migrações existe na academia
  ([He et al.](https://hehao98.github.io/files/2021-migration-empirical.pdf),
  [CMU](https://www.cs.cmu.edu/~ckaestne/pdf/icse25_abandonment.pdf)), não como produto no
  momento da escolha.
- **Radar do mundo**: vigiar só as fontes externas que as premissas de uma decisão citam
  (versões, avisos, licenças) e perguntar se a decisão ainda vale. Monitores genéricos de
  dependência existem; nenhum sabe o porquê da escolha.
- **O agente decidiu por você**: no fim da sessão, as suposições que o agente fez sem perguntar,
  para confirmar ou rejeitar com um clique. A extração existe como pesquisa
  ([AssumptionMiner](https://arxiv.org/abs/2607.22898)), sem memória entre sessões.
- **Café da manhã de decisões**: prever à noite as perguntas que as tarefas do dia vão gerar e
  respondê-las de manhã num cartão de um minuto.
- **Xemnas Wrapped**: o ano em decisões, gerado localmente, para autoavaliação e para
  compartilhar se a pessoa quiser.

Descartadas na pesquisa: a linha ligada à conversa que a produziu já existe
([Agent Trace](https://cognition.com/blog/agent-trace)); o "revisor sombra" com o gosto da
pessoa teve ganho pequeno e inconsistente num estudo com 206 sessões
([arXiv:2608.10319](https://arxiv.org/abs/2608.10319v1)); teste de arquitetura gerado da
decisão é padrão conhecido (fitness functions) e cabe nos verificadores da ideia 1.

## O que não fazer

- Competir em "memória entre sessões" ou "memória compartilhada entre agentes": já é recurso de
  fábrica.
- Transformar o verificador em revisor de IA genérico: perde o argumento de "só o que uma
  pessoa decidiu" e o custo zero.
- Travar escrita de agentes: avisar é barato; serializar mata o paralelismo.
