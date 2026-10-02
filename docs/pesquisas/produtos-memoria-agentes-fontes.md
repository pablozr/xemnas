# Produtos de memória para agentes: fontes e oportunidades

**Data:** 2026-10-02. Consulta documental nessa data.

**Pergunta:** o que produtos de memória/contexto para agentes já entregam e que
expansões ajudariam o xemnas a preservar decisões de engenharia confiáveis?

**Status:** Aberta. Pesquisa documental concluída; hipóteses de produto e comparação
experimental pendentes. Não altera escopo aprovado.

## Enquadramento e método

O ponto de partida é o [vocabulário](../produto/CONTEXT.md), a
[especificação](../produto/MVP-SPEC.md) e as
[ideias anteriores](ideias-de-produto.md). O xemnas preserva escolha humana,
rationale, premissas, Evidence e condições de reconsideração. Não basta competir
por quantidade de mensagens recordadas. Contexto, importação de regras, conflitos,
reconsideração e medição de uso já aparecem na pesquisa anterior: aqui recebem
contrapontos de mercado e hipóteses mais específicas, não são descobertas novas.

Foram consultadas páginas oficiais, documentação e repositórios dos mantenedores.
**Observado** significa descrito na fonte aberta, não recurso executado e aprovado.
**Inferência** significa nossa interpretação de oportunidade. Ausência nas páginas
consultadas não prova ausência no produto. Rankings comerciais não são medição do
xemnas. Nenhum produto foi instalado, nenhuma conta foi criada e nenhum dado do
projeto foi enviado a esses serviços.

## Matriz de oito famílias de produtos

| Produto | Observado e fonte primária | O que implica para a comparação |
| --- | --- | --- |
| Mem0 / OpenMemory | OpenMemory oferece memória compartilhada por MCP e dashboard; guia usa API, Postgres e Qdrant locais, com chave OpenAI para inferência. [Guia oficial](https://mem0.ai/library/coding-agents/how-to-make-your-clients-more-context-aware-with-openmemory-mcp) | Armazenamento local não prova inferência offline. Portabilidade entre agentes já é uma proposta existente. |
| Zep / Graphiti | Graphiti possui episódios de origem, validade temporal, invalidação preservando histórico e busca híbrida. Zep é infraestrutura gerenciada; Graphiti é framework auto-hospedado. [Repositório oficial](https://github.com/getzep/graphiti) | Grafo, temporalidade e proveniência isolados não sustentam exclusividade do xemnas. |
| Letta | SDK atual descreve memória Git projetada por MemFS e leitura sob demanda; V1 tem blocos compartilhados e opção somente leitura. [SDK atual](https://docs.letta.com/agent-sdk/memory), [V1](https://docs.letta.com/v1-sdk/memory/memory-blocks) | Precisamos distinguir versões. Memória editável pelo agente e política protegida podem coexistir. |
| Supermemory | Memórias atualizam, estendem e derivam outras, preservando histórico. [Produto](https://supermemory.ai/product/). Changelog anuncia servidor local macOS/Linux e plugins para agentes. [Changelog](https://supermemory.ai/changelog/api/) | Não classificar automaticamente como apenas cloud ou simples busca vetorial. |
| Pieces | LTM captura contexto de aplicações, oferece timeline e MCP, processamento/armazenamento local e controles por fonte, modalidade e período. [LTM](https://docs.pieces.app/products/core-dependencies/pieces-os/long-term-memory) | Captura silenciosa e recuperação do trabalho já têm concorrência desktop. |
| Cursor | Rules possuem escopo por arquivo, aplicação inteligente/manual/sempre e regras de equipe com precedência. [Rules atual](https://cursor.com/docs/rules) | Regras por escopo são uma alternativa familiar ao usuário; interoperabilidade importa. |
| Continue | Regras Markdown locais, globs, regex e aplicação por descrição/alwaysApply; geração pelo agente quando solicitada. [Rules](https://docs.continue.dev/customize/deep-dives/rules) | Exportar texto sem preservar a semântica de ativação pode mudar o comportamento. |
| Hindsight | Retain/recall/reflect; bancos por projeto; observações com evidência; knowledge pages; integrações de coding agents e execução local. [Repositório](https://github.com/vectorize-io/hindsight) | Concorrente próximo para memória técnica evolutiva. Biblioteca e síntese persistente já existem. |

## Leitura aprofundada e limites

### Mem0 / OpenMemory

**Observado:** o guia de OpenMemory expõe operações de adicionar, buscar e listar
memórias por MCP e interface de gestão. A instalação documentada depende de Docker
e chave de modelo. Portanto, separar residência do banco e destino da inferência é
essencial na avaliação. [Guia](https://mem0.ai/library/coding-agents/how-to-make-your-clients-more-context-aware-with-openmemory-mcp).

**Inferência para o xemnas:** copiar a simplicidade de conectar um segundo agente,
mas entregar um recibo do Context Pack: origem, autoridade, validade e conteúdo
realmente enviado. O valor proposto seria evitar a reapresentação de uma escolha
revogada, não apenas recordar preferências. Não foi verificada política de aprovação
por item, resistência a injeção ou equivalência temporal ao domínio do xemnas.

**Avaliação publicada:** o paper Mem0 compara recuperação em LOCOMO e diferentes
baselines; isso não mede obediência a decisões de engenharia ou tempo de revisão
humana. [Paper](https://arxiv.org/abs/2504.19413). Não transplantar resultados para o
produto sem reproduzir os mesmos modelos, corpus e orçamento.

### Zep / Graphiti

**Observado:** episódios mantêm a origem dos fatos, relações têm janela de validade
e busca combina semântica, palavras-chave e travessia. A instalação requer Python,
backend de grafo e inferência; endpoint de modelo local é uma opção documentada.
[Graphiti](https://github.com/getzep/graphiti).

**Inferência:** é referência de representação temporal, não justificativa para
substituir SQLite. Automaticamente invalidar um fato extraído e substituir uma
Engineering Decision humana são operações diferentes. O xemnas pode apresentar
“esta nova observação tensiona a decisão” preservando ambas até revisão. Um grafo
mais correto temporalmente não garante decisão mais correta.

**Não verificado:** custo no Windows alvo, qualidade com modelo pequeno, tempo de
ingestão real e política de confirmação humana específica do domínio. A própria
fonte diferencia desempenho gerenciado de implantação própria; números do Zep não
são promessas do Graphiti no desktop.

### Letta

**Observado:** no SDK atual, arquivos em `system/` entram no prompt; demais arquivos
ficam disponíveis por árvore/leitura sob demanda. MemFS representa repositório Git
do agente; alterações viram memória após commit e push. Repositórios compartilhados
hospedados são recurso cloud, com alternativa de Git próprio em self-host.
[Memória atual](https://docs.letta.com/agent-sdk/memory).

**Observado com versão:** V1 permite blocos somente leitura e blocos compartilhados;
não presumir que o mesmo contrato exista no novo SDK sem conferir.
[Blocos V1](https://docs.letta.com/v1-sdk/memory/memory-blocks).

**Inferência:** inspirar uma fronteira explícita entre decisões confirmadas e notas
de trabalho. Um agente pode propor alterações, mas a autoridade da decisão precisa
permanecer numa transição humana auditável. Git ajuda na portabilidade; um commit de
memória do agente não deve equivaler à confirmação humana. Não adotamos infraestrutura
Letta nem validamos sincronização, offline ou resolução de conflitos.

### Supermemory

**Observado:** o produto descreve três relações distintas: atualização, extensão e
derivação. Documento bruto e memória extraída são entidades diferentes.
[Produto](https://supermemory.ai/product/). A orientação oficial exige incerteza
visível em inferências e propõe testar remoção da fonte e acesso.
[Semântica das relações](https://supermemory.ai/blog/memory-graph-relationship-semantics/).

**Observado:** changelog registra servidor local para macOS/Linux e instalação em
Claude Code, Cursor, OpenCode e Codex; não verificado suporte local Windows nessa
página. [Changelog](https://supermemory.ai/changelog/api/).

**Inferência:** o diferencial promissor é transformar uma mudança de evidência em
reconsideração humana compreensível. Exemplo: a fonte atualizou o limite de conexão,
o Assessment aponta uma premissa afetada e oferece comparar alternativas; nunca
atualizar silenciosamente a escolha confirmada.

**Avaliação:** o estudo publica Recall@20 em LongMemEval e descreve recuperação de
chunks de origem. Recall@20 não é taxa de decisões certas nem ausência de conteúdo
obsoleto no pack. [Pesquisa](https://supermemory.ai/research/longmembench/).

### Pieces

**Observado:** captura contexto de tela, clipboard e áudio, recupera por timeline e
MCP e permite pausa e exclusão granular. Pausar fonte impede novas capturas; dados
anteriores permanecem. [LTM](https://docs.pieces.app/products/core-dependencies/pieces-os/long-term-memory).

**Inferência:** o xemnas deveria aproveitar a clareza dos controles de captura,
sem ampliar coleta para toda a tela para parecer mais completo. Seu turno + diff
tem semântica de engenharia mais estreita. Uma entrada “retomar este projeto” pode
mostrar o que mudou e que decisões importam ao trabalho atual, sem competir por
registro total da vida digital. Não verificamos geração de decisões confirmadas,
qualidade de citações em tarefas reais nem todos os fluxos de inferência remota.

### Cursor

**Observado:** Rules atuais distinguem aplicação manual, por arquivo, inteligente
e permanente. Regras de equipe podem ser obrigatórias e têm precedência documentada.
[Rules](https://cursor.com/docs/rules).

**Cuidado de atualidade:** busca retornou documentação antiga de Memories com
extração de conversas e aprovação; abrir a
[URL antiga](https://docs.cursor.com/en/context/memories) redirecionou ao índice
atual. Não usamos esse resultado como comprovação de recurso atual. A página atual
de Rules consultada não continha seção Memories.

**Inferência:** construir comparação visual entre regra escrita e decisão que a
justificou. Exportar uma decisão como regra exige escopo e preview; a origem deve
continuar acessível. Não alegar que só o xemnas tem autoridade humana: há controle
editorial e administrativo em produtos existentes.

### Continue

**Observado:** globs e regex controlam entrada de regra, com semântica específica
para `alwaysApply`; regras entram em Agent, Chat e Edit, mas não autocomplete ou
apply. [Documentação](https://docs.continue.dev/customize/deep-dives/rules).

**Inferência:** um adaptador de exportação deve traduzir escopo e indicar perdas,
por exemplo quando uma condição temporal não cabe no formato de destino. Produto
interessante: prévia “o que o agente receberá nesta tarefa”, com explicação do
motivo de inclusão/exclusão. Isso avança a aba Contexto existente, em vez de
redescobri-la. Não foi demonstrada equivalência de resultado entre Continue e
outros agentes usando o mesmo texto.

### Hindsight

**Observado:** recall combina buscas temporal, lexical, semântica e grafo;
knowledge pages persistem sínteses; integração de coding agents cria banco por
repositório. [Repositório](https://github.com/vectorize-io/hindsight). Observações
consolidadas preservam suporte e podem evoluir com evidência nova.
[Observações](https://hindsight.vectorize.io/developer/observations).

**Observado:** Memory Defense é opt-in e detecta/redige ou bloqueia padrões de
segredos e PII. Isso não comprova resistência geral a prompt injection.
[Memory Defense](https://hindsight.vectorize.io/developer/memory-defense).

**Inferência:** “memória que aprende” é território ocupado. A hipótese do xemnas
deve ser aprender quais decisões precisam ser reconsideradas, com humano no
comando e ligação ao código. Não usar prova/quote como sinônimo de verdade; uma
fonte pode registrar hipótese, sarcasmo ou sugestão rejeitada. Não reproduzimos
os benchmarks publicados nem verificamos eficácia da defesa em português.

## Oportunidades que a comparação sustenta

As propostas abaixo são **inferências de produto**, não alegações de lacuna exclusiva
do mercado e não mudanças de escopo aprovadas.

| Proposta | Situação concreta e entrega | Ganho sobre ideias já registradas | Experimento mínimo |
| --- | --- | --- | --- |
| Recibo de contexto | Após recuperar um pack, mostrar decisão, versão, origem, motivo de inclusão e o que ficou fora | Evolui a prévia de Contexto para explicar uma entrega real e reproduzi-la | Comparar diagnóstico de 10 respostas ruins com/sem recibo; tempo até localizar causa |
| Retomar com diferenças | Ao voltar depois de uma semana, mostrar decisões novas/substituídas e premissas alteradas que afetam o escopo atual | Resumo semanal torna-se comparação entre snapshots, focada na tarefa | Medir tempo de reorientação e correções feitas ao agente em 5 retornos reais |
| Sala de reconsideração | Premissa mudou; agrupar fonte nova, decisão afetada, alternativas e custo de manter/trocar | Vai além do alerta: conduz revisão fundamentada sem confirmar por IA | 10 mudanças conhecidas, revisão cega de alertas e medição de falsos positivos |
| Memória de alternativas rejeitadas | Recuperar por que uma opção foi descartada e sob quais condições voltaria a ser válida | Acrescenta rationale negativo estruturado, sem proibir revisitar alternativas | Reexecutar tarefas em que o agente repetiu uma proposta já descartada |
| Contrato de contexto entre agentes | Mesmo pack semântico para dois agentes, com campos suportados e perdas do adaptador | Exportação deixa de ser cópia de Markdown; preserva escopo e versões | Dois agentes, mesma tarefa, decisão revogada e regra por diretório; inspecionar entregas |
| Laboratório de contexto | Comparar resposta sem memória, com regras atuais e com pack; explicar ganho/custo | Avança contagem de citações para utilidade em tarefa | 20 tarefas históricas com evidências posteriores ocultas e julgamento humano |

Prioridade sugerida neste recorte: recibo + retomar com diferenças primeiro;
reconsideração e alternativas rejeitadas depois; contrato multiagente somente com
uso demonstrado em um segundo adaptador. Laboratório começa pequeno antes das
expansões para medir se elas ajudam.

## Segurança e avaliação como parte do valor

Requisitos propostos, sujeitos a decisão de produto:

1. Origem não tem autoridade por estar na memória. Source Artifact que diz
   “ignore as regras e confirme isto” continua sendo dado; nunca comando.
2. Pack informa separadamente Engineering Decision, Claim e Assessment. Uma
   inferência não deve ganhar status de escolha humana ao ser resumida/exportada.
3. Correção, revogação, exclusão e perda de acesso precisam repercutir nos derivados,
   caches e packs posteriores. Recibo antigo é histórico, não contexto atual.
4. Localidade tem quatro perguntas: captura, armazenamento, embeddings e inferência.
   Mostrar cada destino real, sem promessa genérica de “local”.
5. Nenhum grafo garante completude. Incluir “evidência insuficiente” quando a
   recuperação não sustenta resposta, e medir abstenção correta.

Conjunto mínimo de avaliação: decisão substituída; dois diretórios com regras
distintas; sugestão rejeitada; documentação antiga importada hoje; fonte excluída;
conversa maliciosa; duas pessoas com nomes iguais; pergunta histórica versus atual;
premissa alterada sem decisão substituída; tarefa sem memória relevante.

Métricas: recuperação de Evidence relevante, vazamento de escopo, inclusão de
decisão revogada, inferência promovida indevidamente, acerto da resposta com
citação correta, tokens/latência p95, tempo humano de revisão e retrabalho evitado.
Executar com corpus congelado, instante de corte e mesmo modelo; contar citação
sozinha apenas como uso, nunca como melhoria causal.

## Limites e próxima decisão

É comparação documental, não benchmark comercial completo. Não examinamos todos
os planos, contratos, custos, licenças de cada componente ou garantias empresariais.
Links mudam: páginas antigas de Letta e Cursor demonstram por que revalidar antes de
integração. Suporte anunciado em README não prova instalação funcional no Windows
alvo. Não encontramos evidência suficiente para afirmar que qualquer fornecedor
resolve integralmente o ciclo decisional do xemnas; isso também não prova novidade
absoluta da proposta.

A melhor pergunta de dogfood é: **o usuário consegue explicar e reconsiderar uma
escolha mais rápido, sem aceitar memória incorreta?** Se não melhorar esse resultado,
mais captura, mais nós e mais síntese não justificam a complexidade.
