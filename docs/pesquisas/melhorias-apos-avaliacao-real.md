# Melhorias do Xemnas após a avaliação real

**Data:** 03/10/2026.
**Pergunta:** quais mudanças devem tornar o Xemnas mais confiável, fácil de usar e útil, considerando as pesquisas anteriores e os retestes no ripgrep?
**Status:** Aberta. Proposta priorizada; nenhuma correção de produto foi implementada nesta síntese. Não altera o escopo aprovado.

A recomendação é melhorar primeiro a qualidade do ciclo captura → revisão → contexto. Os testes comprovaram operações importantes do núcleo, mas também mostraram contexto irrelevante, perda de qualificadores, esforço manual e estados difíceis de interpretar. A evolução mais promissora é ajudar a pessoa a retomar uma tarefa e perceber quando uma premissa merece revisão. Essa evolução depende de uma memória confiável.

## Evidência e prioridade

A base principal é a [avaliação em repositório real](../operacao/avaliacao-repositorio-real.md), incluindo cinco críticos Luna medium, retestes com provider real e [auditoria do core](../operacao/avaliacao-repositorio-real/core/core-audit.md). As [relações pela GUI](../operacao/avaliacao-repositorio-real/core/relacoes-gui-final.md) e a [retomada](../operacao/avaliacao-repositorio-real/core/retomada-release.md) prevalecem sobre críticas anteriores que não acompanharam esses testes.

As propostas de expansão vêm de [oportunidades de memória decisional](oportunidades-produto-memoria-decisional.md) e [vinte ideias sobre dores de desenvolvedores](20-ideias-dores-reais-devs.md). A escolha de busca local deve respeitar [memória semântica local first](memoria-semantica-local-first.md); acabamento e escala seguem [plano de design](plano-design-vitrine.md) e [pesquisa de renderização](escalabilidade-renderizacao-fontes.md). Esta síntese não atualiza comparações de fornecedores, preços ou modelos externos.

Cada proposta distingue três bases: **observado** é um resultado executado; **código** é uma lacuna encontrada por inspeção, sem incidente necessariamente reproduzido; **hipótese** é uma utilidade ainda não demonstrada. P0 significa proteção da confiança antes de ampliar uso; P1 é o próximo ciclo de qualidade; P2 é expansão ou acabamento posterior. Prioridade e esforço são avaliações qualitativas, não estimativas de prazo.

## Correções e fortalecimento do produto atual

### 1 Proteger todos os envios ao provider

**Prioridade:** P0. **Base:** código. **Frente:** backend. **Esforço relativo:** médio.

A edição humana pode introduzir conteúdo que não passou pela redação da ingestão; o caminho de sugestões de regras interpola o texto da decisão. Não foi observado vazamento. A melhoria é tornar a preparação do conteúdo externo uma fronteira comum: redação, limites, configuração e consentimento válidos no instante da chamada. A prévia deve corresponder ao conteúdo que será enviado, com retenção explícita caso se acrescente auditoria de payload.

**Concluído quando:** um segredo sintético inserido depois da captura não chega ao provider por nenhum caminho coberto; revogar consentimento ou mudar o destino invalida a próxima chamada; uma configuração válida continua funcionando. Testar análise, sugestões, revisão e Visão, sem usar segredos reais.

### 2 Preservar autoria e escopo do conhecimento

**Prioridade:** P0. **Base:** observado. **Frente:** backend e front. **Esforço relativo:** médio.

O resumo de uma fixture perdeu a ressalva de simulação local presente na evidência. Tornar visíveis autoria, fonte, abrangência e limite de validação na revisão, no detalhe e no contexto enviado. Uma escolha do avaliador não deve parecer orientação dos mantenedores. Separar explicitamente escolha, motivo, alternativa rejeitada e o que realmente foi testado. Se campos estruturados novos forem necessários, registrar a mudança de contrato antes de implementá-los.

**Concluído quando:** os casos local, simulado e confirmado pelo projeto mantêm essas diferenças na extração, edição, derivação de regras e exportação; outro agente consegue explicar o alcance correto sem reler toda a sessão.

### 3 Melhorar relevância da busca e permitir ausência de resposta

**Prioridade:** P1, primeira melhoria de valor cotidiano. **Base:** observado e código. **Frente:** backend. **Esforço relativo:** médio, com experimento prévio.

A consulta sobre cor do terminal trouxe uma decisão sem relação suficiente; a consulta em inglês perdeu informação encontrada em português. A busca lexical com OR explica um risco de correspondência fraca, mas não estabelece sozinha a melhor solução. Construir um corpus rotulado, ajustar seleção e explicar por que cada item entrou. Para contexto automático, permitir não entregar itens quando a evidência de relevância é insuficiente. A busca exploratória pode oferecer resultados fracos, identificados como tal.

Comparar primeiro a base atual com melhorias lexicais. Experimentar busca híbrida e reordenação local somente com o mesmo corpus e orçamento de memória/latência. Não escolher modelo ou motor antes desse resultado.

**Concluído quando:** os negativos conhecidos deixam de poluir o contexto; consultas equivalentes em português/inglês recuperam a informação esperada no corpus; precisão, omissões e latência são publicadas contra a base anterior. Fixar metas antes de ajustar o algoritmo e reservar exemplos que não participem da calibração.

### 4 Explicar o destino de cada captura

**Prioridade:** P1. **Base:** observado. **Frente:** backend e front. **Esforço relativo:** baixo a médio.

Uma fila vazia pode significar detalhe corretamente descartado, job em andamento ou falha. Mostrar o percurso da captura e a razão do resultado: recebida, analisando, sem conhecimento durável, candidato disponível ou erro recuperável. Categoria, motivo e fonte devem ajudar mais que um número de confiança. Identificar claramente dados demo/Fake.

**Concluído quando:** uma pessoa distingue rotina descartada, candidato arquitetural e provider indisponível sem abrir logs ou consultar SQLite. Nada na interface sugere que alta confiança equivale a teste executado.

### 5 Corrigir os defeitos de interface observados

**Prioridade:** P1, entrega curta. **Base:** observado. **Frente:** front. **Esforço relativo:** baixo a médio.

Corrigir o toast de regra que anuncia decisão criada; evitar texto de sugestões sobre botões; atualizar fila e diagnóstico após conclusão do job; alinhar a descrição de documentos ao fato de que eles podem gerar candidatos. Reproduzir também a colisão de dica/filtros registrada na pesquisa visual. Ajustar linguagem e layout dentro do Quiet Glass existente.

**Concluído quando:** os estados das capturas 37/40/41 e da pesquisa visual ficam legíveis em janela normal e compacta, nas duas paletas exigidas; ações continuam alcançáveis por teclado; status corresponde ao resultado persistido. Ler VISUAL-IDENTITY antes de editar telas.

### 6 Conduzir até a primeira memória útil

**Prioridade:** P1. **Base:** dificuldade observada; percurso novo é proposta. **Frente:** front, integração e distribuição. **Esforço relativo:** médio.

Reutilizar Projetos, configuração e diagnóstico para guiar cadastro → conexão → captura recebida → revisão → recuperação. Cada etapa precisa de confirmação real e uma ação seguinte. Mostrar último recebimento e erro recuperável, evitando declarar integração ativa apenas porque foi configurada.

A distribuição ZIP e os scripts de instalação já existem. O trabalho é validar e simplificar o percurso release atual em instalação isolada, incluindo instalação do plugin e upgrade, sem exigir toolchain do usuário. Rebuild que abriu no host de avaliação não prova instalação generalizável.

**Concluído quando:** um usuário começa pelo pacote e recupera uma primeira decisão sem editar wrappers; falhas de conexão/provider indicam causa e próxima ação. Medir tempo e pontos de abandono, sem anunciar uma meta de minutos ainda não validada.

### 7 Comparar regras do mesmo escopo na revisão

**Prioridade:** P1. **Base:** observado e código. **Frente:** backend. **Esforço relativo:** médio.

As regras opostas foram enviadas separadamente e o modelo pediu a regra ausente. Ampliar a seleção de unidades para incluir regras potencialmente incompatíveis ligadas ao mesmo escopo, mesmo quando não há par de decisões. Mostrar o que foi revisado e o que ficou de fora. Comparação com modelo continua consultiva e precisa de evidência; zero achados não certifica consistência.

**Concluído quando:** a fixture oposta é comparada em conjunto; regras compatíveis não geram conflito só por compartilhar termos; orçamento limitado declara omissões. Nenhuma decisão é retirada automaticamente.

### 8 Reduzir sugestões duplicadas e explicitar conhecimento antigo

**Prioridade:** P1. **Base:** observado. **Frente:** backend e front. **Esforço relativo:** médio.

Comparar regras propostas com as já vigentes antes de apresentar outra confirmação. Oferecer manter, relacionar ou descartar a proposta com evidência. Quando uma decisão é substituída, explicar quais regras derivadas continuam vigentes e quais merecem revisão. Substituição não autoriza retirar automaticamente todas as regras antigas.

**Concluído quando:** a fixture não produz duplicatas equivalentes sem indicação; regras diferentes preservam distinção de escopo; o usuário consegue conferir a linhagem e resolver o caso deliberadamente.

### 9 Tornar adoção consistente e recuperável

**Prioridade:** P1. **Base:** código; falha parcial não reproduzida. **Frente:** backend. **Esforço relativo:** médio a alto.

A confirmação precede a gravação dos vínculos. Escolher uma transação única ou uma operação persistida e retomável, conforme as fronteiras de armazenamento; conferir seleção contra uma prévia identificada e ainda válida. A interface precisa receber um resultado que descreva o estado real, inclusive recuperação de operação parcial.

**Concluído quando:** falhas injetadas entre confirmação e vínculos não produzem sucesso enganoso nem decisões duplicadas no retry; prévia desatualizada é recalculada ou recusada; sugestões de outro escopo/projeto não são aceitas.

### 10 Endurecer replay e checkpoint

**Prioridade:** P1. **Base:** código; replay idêntico já passou. **Frente:** backend e adapter. **Esforço relativo:** médio.

Associar a chave idempotente à identidade do envelope e rejeitar reuso com conteúdo diferente; impedir que captura atrasada faça o checkpoint retroceder. Definir a identidade sobre uma representação estável, considerando redação e versão do contrato. Acrescentar provas de crash e estados rejeitado/travado que faltaram à rodada.

**Concluído quando:** replay idêntico permanece aceito sem duplicação, payload divergente gera conflito explícito, recebimento fora de ordem preserva o checkpoint correto e reinício não perde captura confirmada.

### 11 Corrigir a semântica de consultas históricas

**Prioridade:** P1 antes de prometer contexto histórico fiel. **Base:** código; comparação temporal não executada. **Frente:** backend. **Esforço relativo:** médio.

O filtro temporal usa validade, mas o conteúdo vem da versão atual da decisão. Selecionar a revisão vigente na data consultada ou limitar explicitamente o contrato oferecido. Fazer a mesma distinção em exportação e recibos; não apresentar texto futuro com status passado.

**Concluído quando:** consultas antes/depois de edição e substituição retornam texto, versão e status coerentes, com casos nos limites de data. Migração e compatibilidade seguem a política existente.

### 12 Explicar a entrega de contexto

**Prioridade:** P1. **Base:** auditoria existente; enriquecimento é hipótese de UX. **Frente:** backend, front e adapter. **Esforço relativo:** médio.

Evoluir o registro atual para explicar selecionado, omitido por orçamento, desatualizado e entregue. Guardar versão da política e referências suficientes para reconstruir a seleção. Um recibo de entrega não prova que o agente leu ou seguiu a regra. Conteúdo integral só pode ser retido com política explícita; começar por metadados e versões.

**Concluído quando:** os cinco cenários do recibo proposto na pesquisa anterior permitem localizar a etapa da falha; a UI não chama de usado o que só foi enviado. O mesmo instrumento sustenta a medição de relevância.

### 13 Tornar Mapa e Visão explicativos e medir escala

**Prioridade:** P2; clareza pode acompanhar as correções de interface. **Base:** observado e hipóteses de escala. **Frente:** front e backend. **Esforço relativo:** médio.

Mostrar responsabilidades e fontes onde existirem; identificar componentes apenas descobertos por diretório; indicar etapas sem vínculo, cobertura limitada e idioma escolhido. Preservar a navegação fluxo → entidade, que passou no reteste. Medir listas/grafo com corpus maior antes de escolher virtualização e níveis de detalhe. A rodada com onze entidades não comprovou desempenho em escala.

**Concluído quando:** o mapa explica o conhecimento registrado sem sugerir relações inexistentes; a Visão distingue fontes e lacunas; benchmark integrado publica latência e memória nos tamanhos escolhidos, respeitando o alvo de 8 GB/CPU.

### 14 Completar a remoção de dados do projeto

**Prioridade:** P1 de manutenção do núcleo. **Base:** observado. **Frente:** backend. **Esforço relativo:** baixo a médio.

A purga no snapshot manteve integridade e o clone, mas deixou dois jobs concluídos. Definir se são histórico retido intencionalmente ou resíduo a eliminar. Fazer jobs em execução cooperarem com remoção, sem recriar dados depois dela. O fluxo destrutivo mantém confirmação explícita.

**Concluído quando:** o resultado da purga corresponde à política documentada, não deixa dados do projeto fora dela e continua correto com worker concorrente. Nenhum arquivo do repositório é apagado por essa operação.

## Evoluções que podem agregar valor

### 15 Retomada orientada à tarefa

**Prioridade:** P2, primeira aposta de recurso após a qualidade básica. **Base:** hipótese apoiada nas capacidades atuais. **Esforço relativo:** baixo a médio.

Ao voltar ao projeto e informar a tarefa, apresentar o que mudou desde um marco explícito, o que continua valendo e o que merece conferência. Reutilizar Contexto e Visão; uma nova aba não é necessária. Depende da seleção relevante, do histórico coerente e da preservação de escopo das melhorias 2, 3 e 11.

**Prova de valor:** comparar tarefas diferentes com retomada atual e proposta, controlando o aprendizado; medir tempo para explicar restrições e fontes, além de erros. Avançar se reduzir esforço sem piorar correção. Isso é mais útil que apenas adicionar outro resumo geral.

### 16 Radar de premissas e ensaio de mudança

**Prioridade:** P2, aposta de diferenciação. **Base:** hipótese. **Esforço relativo:** médio a alto, em etapas.

Começar com dez condições de reconsideração associadas manualmente a fontes. Ao detectar mudança numa fonte, apresentar a premissa, o trecho novo e uma pergunta de revisão. Mudança de arquivo é sinal, não prova de premissa inválida. Depois, permitir ensaiar uma alternativa e consultar dependências registradas, inferências citadas e informação ausente, sem alterar decisões vigentes.

**Prova de valor:** executar primeiro o piloto manual previsto na pesquisa; medir alertas úteis, repetidos e ignorados. Só criar detector se a triagem compensar. O ensaio deve reproduzir caminhos registrados e não prometer impacto completo quando o grafo é parcial.

### 17 Preparar uma tarefa transferível e verificável

**Prioridade:** P2, experimento curto depois da retomada. **Base:** hipótese da pesquisa de dores. **Esforço relativo:** médio.

Compor um pacote com objetivo, decisões aplicáveis, caminhos descartados, passos de reprodução e critérios de validação. Aproveitar exportação e contexto; começar como composição manual, sem automatizar outro agente. Separar teste executado de resultado esperado e de check bloqueado pelo ambiente.

**Prova de valor:** um segundo agente retoma a tarefa sem repetir uma investigação já documentada e devolve evidência verificável. Comparar com um ADR e instrução de tarefa bem escritos; se não diminuir trabalho, manter somente as partes úteis dentro do fluxo atual.

## Sequência recomendada

| Etapa | Entregas | Por que nessa ordem | Critério para avançar |
| --- | --- | --- | --- |
| 1 Confiança e correções visíveis | 1, 2, 4, 5 | Protege conteúdo e faz o usuário entender o que aconteceu | Fixtures de envio/escopo passam; os defeitos visuais observados não reaparecem |
| 2 Contexto e primeira utilização | 3, 6, 12 | Ataca a utilidade diária e explica falhas de seleção | Primeira memória recuperada pelo pacote; corpus mostra qualidade superior à base |
| 3 Consistência do conhecimento | 7, 8, 9, 10, 11, 14 | Evita duplicação, cobertura enganosa e estado parcial | Casos de contradição, falha, replay, tempo e remoção passam |
| 4 Valor e escala | 13, 15; piloto manual de 16; experimento de 17 | Expande sobre uma base confiável e mede benefício | Menos esforço com correção preservada e comportamento aceitável em corpus maior |

As frentes de integridade da etapa 3 podem caminhar em paralelo com a etapa 2. Para cada mudança, abrir um ticket específico na fase apropriada e conferir tickets existentes, evitando reabrir distribuição, captura ou relações como se nada estivesse implementado. Backend e front trabalham em branches/sessões distintas; UI chama casos de uso. Cada alteração deve ter validação proporcional e commit próprio.

Minha primeira entrega proposta é o conjunto **destino da captura + toast correto + layout legível + atualização de status**, acompanhado da proteção de envios e do escopo. Ele reduz fricção observada e facilita medir a melhoria de busca seguinte. Esta ordem não transforma tarefas pequenas em substitutas da correção do núcleo.

## O que adiar

Adiar biblioteca global com OCR amplo, chat livre, muitas integrações, contexto por branch e decoração adicional. São direções possíveis das pesquisas anteriores, mas não corrigem os problemas que impediram confiança cotidiana. Contexto por branch volta à pauta quando houver casos reais de decisões de experimento confundidas com decisões vigentes. Embeddings voltam como experimento de relevância, não como promessa automática de qualidade.

## Medir se o app ficou melhor

Manter um conjunto fixo de regressões derivado do ripgrep e um conjunto separado de exemplos inéditos. Registrar precisão de contexto, omissões relevantes, fidelidade de fonte/escopo, ações manuais por decisão, tempo humano efetivo, tempo de máquina e recuperação de falhas. Contagem de decisões e tamanho do grafo não demonstram valor.

Comparar tarefas equivalentes com sessão sem memória, ADR bem escrito e Xemnas, usando ordens e tarefas distintas para reduzir aprendizado. Os cinco agentes Luna contribuíram com perspectivas úteis, mas não substituem usuários humanos; a rodada não comprovou produtividade nem demanda comercial. O critério final é alguém recuperar ou verificar uma mudança corretamente com menos esforço de manutenção.

## Limites e incorporação

Nenhuma melhoria acima está implementada por esta nota. Não há estimativa de calendário, certificação de segurança nem nova escolha de fornecedor. Incidentes observados e lacunas estáticas permanecem separados. Ao implementar um assunto, levar seus contratos e regras duradouros para arquitetura, ADR ou identidade visual; retirar desta pesquisa a proposta incorporada e manter o índice atualizado. Quando a síntese estiver inteiramente incorporada, removê-la conforme a regra de pesquisas.
