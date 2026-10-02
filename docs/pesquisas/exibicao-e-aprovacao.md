# Exibir sem sobrecarregar e aprovar sem cansar

Data: 2026-10-02.

**Pergunta:** como mostrar ao desenvolvedor o que o Xemnas sabe de um jeito que valha
a atenção dele, sem camadas nem avisos demais, e como deve funcionar a aprovação,
incluindo um modo em que a IA decide sozinha?

**Status:** Aberta. Pesquisa documental (estudos, documentação de produtos e blogs
técnicos) mais uma leitura do código e das telas atuais. Nada foi implementado nem
testado com pessoas; as propostas são hipóteses para o dogfood. Este documento
complementa `dores-dev-cognicao-fontes.md` (retomada e interrupção) e
`dores-dev-ia-verificacao-fontes.md` (autonomia e confiança), que já cobrem a base
cognitiva; aqui entram exibição e aprovação.

## Resposta curta

1. **A melhor tela do Xemnas é a que o desenvolvedor quase não precisa abrir.** O
   valor acontece quando o agente recebe o contexto certo; a interface é a
   manutenção disso. Cada superfície deve ser pensada como "quanto de atenção isto
   custa e o que devolve", não como "o que mais dá para mostrar".
2. **Um modo totalmente automático, ligado por uma chave, é a decisão errada para
   começar.** A evidência mostra o contrário do que a intuição pede: mais pedidos de
   aprovação não deixam mais seguro, deixam o revisor carimbando. O caminho é um
   modo automático **por faixas**, que só se abre depois de medir se a confiança do
   extrator prevê o que o próprio desenvolvedor aceitaria.
3. **Antes de mexer no visual, vale reorganizar por tarefa.** Hoje a navegação
   espelha o modelo de dados (candidatos, decisões, regras, documentos, mapa,
   entregas) e a aprovação está espalhada em quatro lugares. Duas melhorias baratas
   valem mais que mais acabamento: uma fila única de aprovação e uma faixa "desde a
   última vez".

## Evidência

Força: **A** = estudo com medição direta; **B** = estudo ou documento de produto com
ressalvas (pré-publicação, outro domínio); **C** = blog ou relato, só como indício.

| Fonte | O que mostra | Força | Uso aqui |
| --- | --- | --- | --- |
| [Habituation at the Gate](https://arxiv.org/html/2606.22721) (2026) | 400 revisores, 11.429 revisões de PRs de agentes: a aprovação sobe 14,5 pontos entre o primeiro e o décimo decil de experiência, os comentários caem 22% e a latência sobe 3,5 vezes. Padrão de habituação, não de confiança calibrada | B (pré-publicação) | Quem aprova muito aprova pior; é preciso detectar o ritmo e variar a tarefa |
| [Oversight Has a Capacity](https://arxiv.org/abs/2606.08919) (2026) | A atenção do revisor é finita; modelando a fadiga, a segurança vira um U invertido: depois de certa taxa de escalonamento, mais revisão piora. Concordância entre revisores moderada (kappa 0,52) | B | A fila deve ser limitada e priorizada, não exaustiva |
| [Anthropic, autonomia em prática](https://www.anthropic.com/research/measuring-agent-autonomy) | O uso de aprovação automática vai de ~20% das sessões (novatos) a mais de 40% (experientes), mas os experientes interrompem mais (~9% contra ~5% dos turnos): trocam aprovar cada ação por monitorar e intervir. Em tarefas complexas o agente pede esclarecimento mais de duas vezes mais que o humano o interrompe | A (do próprio produto) | O destino natural é monitorar, com visibilidade e desfazer, não aprovar item a item |
| [Bansal et al., CHI 2021](https://dl.acm.org/doi/fullHtml/10.1145/3411764.3445717) | Explicações da IA aumentaram a chance de aceitar a recomendação, certa ou errada; não superaram mostrar apenas a confiança | A | O "motivo" escrito pela IA pode estimular o carimbo; a evidência verificável pesa mais |
| Alertas clínicos ([PMC5387195](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC5387195/) e outros) | Fadiga de alerta: profissionais ignoram a maior parte dos avisos repetidos (relatos de 49% a 96% de substituições) | A (outro domínio) | Cada aviso a mais custa a atenção do próximo |
| [Modelos de adiar para o humano](https://arxiv.org/html/2601.05974) | Duas faixas de confiança: aceita e rejeita sozinho nas pontas, manda para o humano só o ambíguo, com custo de revisão explícito | B | A base técnica das faixas de aprovação |
| [Renovate, redução de ruído](https://docs.renovatebot.com/noise-reduction/) | Agrupar atualizações, mesclar sozinho o de baixo risco com testes passando e agendar a revisão do resto eliminou a fila de PRs por dependência | B (documentação de produto) | O melhor análogo de "automático por faixa" que já existe |
| [Parnin e Rugaber](https://link.springer.com/article/10.1007/s11219-010-9104-9) | Só 10% das sessões retomam a programação em menos de 1 minuto; pistas do contexto reduzem o custo de retomar | A | A faixa "desde a última vez" é uma pista de retomada |
| [Gloria Mark](https://gloriamark.com/attention-span/) | A atenção em uma tela caiu para ~47 s; voltar a uma tarefa interrompida leva ~23 min | A | Orçamento de atenção: avisar pouco, em lote |
| [Shneiderman, 1996](https://www.cs.umd.edu/~ben/papers/Shneiderman1996eyes.pdf) | Visão geral primeiro, filtrar, detalhe sob demanda | A (clássico) | A regra de ordem de toda tela |
| Cowan 2001 (via [resenha de UX](https://careerfoundry.com/en/blog/ux-design/what-is-millers-law/)) | A memória de trabalho segura ~4 blocos, não 7 | A (aplicação a UI é inferência) | Teto de blocos acima da dobra |
| [Calm Technology](https://caseorganic.com/post/principles-of-calm-technology/) | A tecnologia deve pedir a menor atenção possível e usar a periferia | C (princípios de design) | Rótulo para "puxar, não empurrar" |
| [GitHub, 60 milhões de revisões do Copilot](https://github.blog/ai-and-ml/github-copilot/60-million-copilot-code-reviews-and-counting/) | Em 29% das revisões o agente não diz nada; nas demais, ~5 comentários | C | Calar-se é uma escolha de produto válida |

O que a evidência **não** cobre: ferramentas de captura de decisões como o Xemnas.
Quase tudo vem de revisão de código, medicina e agentes de código; a transferência é
plausível, não demonstrada.

## Diagnóstico do Xemnas hoje

Inventário feito no código (`app.rs`, `screens/`):

- **Cinco destinos** no topo (Visão, Revisão, Decisões, Contexto, Mapa), mais Ajustes.
- **Contexto tem oito seções** (Visão geral, Entregas, Testar uma tarefa, Decisões em
  vigor, Regras, Documentação, Revisar conhecimento, Modo de entrega).
- **Mapa tem cinco visões** (Blocos, Grafo, Sugestões, Lente de arquivo, Linha do
  tempo) mais a página de cada componente.
- **Aprovação em quatro lugares:** candidatos na Revisão; sugestões de relação,
  contexto e vínculo no Mapa; propostas de documentos (que viram candidatos); e
  "Revisar conhecimento" no Contexto.
- Cada superfície mostra o que o dado permite, não o que a tarefa pede. Ex.: o
  candidato abre com pergunta, escolha, motivo, mapa, evidência e confiança, nessa
  ordem; o motivo é a explicação da IA e vem antes da evidência.

Isso explica a sensação de "trocentas camadas": são seis conceitos de dado, cinco
destinos e quatro filas, e o desenvolvedor precisa conhecer o modelo para saber onde
agir.

## Regras de exibição propostas

1. **Uma tela, uma tarefa.** Cada destino responde a uma pergunta do desenvolvedor
   ("o que preciso decidir?", "o que o agente sabe?", "o que ele recebeu?", "como é o
   sistema?"), e não a um tipo de dado.
2. **No máximo quatro blocos acima da dobra** e o resto atrás de um clique
   (Shneiderman, Cowan). Listas mostram 5 a 7 itens e "ver mais".
3. **Puxar, não empurrar.** Um único contador de pendências (a Revisão); nada de
   toasts disparados pelo sistema; eventos em lote ("3 novos desde a sua última
   visita"), no máximo uma vez por abertura.
4. **Faixa "desde a última vez"** no topo da Revisão e da Visão: o que mudou
   (candidatos, decisões enviadas ao agente, conflitos). É a pista de retomada.
5. **Ação padrão e desfazer em vez de confirmação.** O que é reversível não pede
   "tem certeza?"; mostra "feito · desfazer" por alguns segundos.
6. **Evidência antes da explicação.** Na Revisão, a fonte verificável (trecho de
   código, sessão) vem primeiro e o motivo escrito pela IA fica recolhido; mostrar a
   confiança como número é mais honesto do que um parágrafo que convence.
7. **Calar-se é permitido.** Candidatos abaixo do limite, duplicados e sugestões de
   baixo valor não aparecem na fila principal (há um filtro "ver também").
8. **Agrupar o que é parecido** (Renovate): vários candidatos da mesma sessão ou do
   mesmo componente viram um item expansível; ação em lote só para a faixa de baixo
   risco.
9. **Forma antes de texto** (já registrado em `analise-do-produto-hoje.md`): número
   com barra ou anel, relação com linha, estado com cor e ícone.
10. **Avançado fora do caminho.** Grafo, linha do tempo e lente de arquivo ficam sob
    um acesso secundário; o caminho comum é fila, conhecimento e entregas.

Fusões concretas a testar no dogfood: Contexto de oito seções para quatro (Resumo,
Fontes com abas de decisões, regras e documentação, Entregas com "Testar uma tarefa",
Ajustes); uma fila única de aprovação que reúna candidatos, sugestões do Mapa e
propostas de documento, com o tipo como filtro.

## Aprovação: do manual ao automático por faixas

### Por que não uma chave "modo automático"

- Aprovar muito não é fiscalizar bem: a aprovação sobe e o esforço cai com a
  experiência (Habituation at the Gate); depois de certo volume, mais pedidos pioram o
  resultado (Oversight Has a Capacity).
- O desenvolvedor experiente já migra sozinho para o automático e passa a monitorar e
  interromper (Anthropic). Ou seja, o automático vai acontecer; o desenho decide se
  ele é seguro.
- No Xemnas o custo de um erro é **contaminação de contexto**: uma decisão errada
  aceita é entregue a todos os agentes seguintes. O erro é reversível (rejeitar ou
  aposentar), mas só se alguém perceber.

### Desenho proposto

**Três faixas, decididas por política e não por humor do modelo:**

| Faixa | O que entra | O que acontece |
| --- | --- | --- |
| A, aceita sozinha | Decisão (não regra) com confiança calibrada acima do limite alto, pelo menos uma fonte verificável, sem conflito nem substituição de decisão existente e tocando poucos componentes | Entra como **provisória**: aparece num livro "aceitas sozinhas" com desfazer e só é enviada ao agente depois de uma janela sem objeção (ou entra marcada com prioridade menor) |
| B, fila | O ambíguo: confiança no meio, evidência fraca, parecido com algo existente | Vai para a fila única, ordenada por valor esperado, com ação padrão sugerida |
| C, sempre humano | Regras (mudam o comportamento do agente), qualquer coisa que contradiga ou substitua uma decisão, pouca evidência e as primeiras decisões de um projeto novo | Nunca é automático |

**Quatro travas:**

1. **Calibração antes de ligar.** A "confiança do extrator" hoje é uma estimativa do
   modelo, não uma probabilidade. Antes de qualquer automático, medir com o histórico
   real de aceitar e rejeitar: dentro de cada faixa de confiança, quanto o
   desenvolvedor aceitou? Se a confiança não prevê a aceitação, o automático fica
   desligado. Isso se faz offline, sem interface, com dados que o app já guarda.
2. **A chave se abre por medição, não por configuração.** Manual (hoje), Assistido
   (só duplicatas e casos óbvios) e Automático; o último só aparece depois de, por
   exemplo, 50 decisões revisadas com concordância acima do limite acordado.
3. **Amostragem às cegas.** Uma em cada dez aceitas sozinhas volta à fila para
   confirmar, sem dizer qual foi a escolha da IA. Se a discordância passar do limite,
   o limite alto sobe sozinho ou o modo cai de volta a Assistido (disjuntor).
4. **Detector de ritmo.** Várias aprovações seguidas em poucos segundos disparam
   "ritmo alto: abrir uma para conferir?" (as auditorias de sequência que o estudo de
   habituação sugere), sem bloquear.

**Medidas do próprio sistema de aprovação** (todas locais): aceitação por faixa de
confiança, tempo mediano por decisão, tamanho das sequências, taxa de desfazer, taxa
de contestação depois de aceitas, e (com o painel de eficácia de
`medir-eficacia-do-contexto.md`) se as decisões aceitas sozinhas ajudam o agente tanto
quanto as revisadas.

## Ordem sugerida

| Passo | O que | Custo | Critério de sucesso |
| --- | --- | --- | --- |
| 1 | Medir a calibração com o histórico de aceitar e rejeitar | Baixo, sem interface | A confiança separa o que foi aceito do que foi rejeitado melhor que o acaso |
| 2 | Faixa "desde a última vez" e um único contador de pendências | Baixo | O desenvolvedor retoma sem procurar o que mudou |
| 3 | Evidência antes do motivo na Revisão; ação padrão com desfazer | Baixo | Menos cliques por decisão sem queda na qualidade |
| 4 | Fila única de aprovação (candidatos, sugestões do Mapa e documentos) | Médio | Um lugar para agir; menos destinos para aprender |
| 5 | Contexto de oito seções para quatro | Médio | Menos navegação para chegar ao mesmo dado |
| 6 | Faixa A com provisória, livro de aceitas sozinhas e amostragem às cegas | Alto; exige backend | Concordância na amostragem acima do limite, taxa de desfazer baixa |

Os passos 2 e 3 são só de interface; 1, 4 e 6 pedem mudança em `crates/` e a sua
confirmação antes.

## O que não sei

- Se a confiança do extrator presta para decidir; é a primeira medição.
- Quanto de "provisória" o desenvolvedor aceita: uma janela de espera pode irritar
  mais que revisar na hora.
- Se as fusões de seção ajudam ou só movem o problema; só o dogfood responde, com uma
  pessoa e sem teste controlado.
- A maior parte das fontes é pré-publicação (arXiv de 2026) ou de outro domínio; os
  números não devem ser tratados como leis para o Xemnas.
