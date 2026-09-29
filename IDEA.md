Visão consolidada do produto

Definição curta

Um aplicativo desktop local que constrói uma memória decisional navegável para cada projeto, mantém uma biblioteca global de conhecimento de engenharia e oferece contexto confiável a assistentes e agentes sem tomar decisões pelo usuário.

Três camadas do produto

1. Memória de cada projeto

Cada projeto possui seu próprio espaço com:





Decision Inbox;



decisões confirmadas;



premissas, restrições, objetivos e convenções;



evidências provenientes de turnos, diffs, arquivos e commits;



contexto atual e histórico;



timeline e grafo de relações;



alertas de contradição e condições de reconsideração.

O OpenCode será a primeira fonte de captura por meio de um adapter. Outros agentes poderão ser integrados depois sem alterar o núcleo.

2. Biblioteca global de conhecimento

O usuário poderá adicionar materiais reutilizáveis entre projetos:





papers;



PDFs de livros;



documentação técnica;



normas e RFCs;



páginas e sites;



anotações próprias.

Cada fonte deve preservar metadados e citações: título, autoria, origem, data, versão e posição do trecho utilizado.

A biblioteca global não será tratada automaticamente como verdade de todos os projetos. Uma fonte pode ser vinculada a projetos ou coleções, e o sistema selecionará apenas os trechos relevantes para cada consulta.

3. Camada de inteligência

Um assistente de engenharia poderá combinar:





a pergunta atual;



contexto válido do projeto;



decisões e premissas anteriores;



estado observado no código;



conhecimento global selecionado;



evidências e citações.

Ele poderá comparar alternativas, identificar conflitos, mostrar trade-offs, apontar informações ausentes e explicar o que mudaria sua recomendação. O assistente produz análises; somente o usuário confirma decisões.

Agentes de implementação poderão consultar a mesma base através de uma API local ou protocolo próprio para agentes, recebendo um Context Pack pequeno e específico para a tarefa — nunca um despejo integral da biblioteca.

Formato recomendado

O produto principal deve ser um aplicativo desktop local, não apenas um plugin.

O aplicativo desktop é o melhor encaixe para a visão macro porque precisa:





organizar vários projetos;



ler arquivos e PDFs locais;



manter banco, índice e modelos locais;



oferecer Inbox, busca, timeline, páginas e grafo;



controlar privacidade e saída de dados;



disponibilizar uma API local para agentes;



funcionar independentemente do OpenCode.

Os plugins serão integrações satélites:

OpenCode plugin ─┐
Outro agente ────┼──> adapters/API local ──> aplicativo desktop
Git/GitHub ──────┘

A interface do desktop pode tecnicamente ser construída com tecnologias web empacotadas, mas essa escolha de framework não precisa ser feita agora. A decisão de produto é: experiência desktop, núcleo local e adapters substituíveis.

Operação local





Não haverá daemon permanente no MVP.



O adapter guarda capturas numa outbox quando o aplicativo estiver fechado.



O motor processa a outbox quando o aplicativo abrir.



Candidatos, decisões e biblioteca ficam inicialmente em armazenamento local.



O sistema não cria commits nem altera o repositório automaticamente.



Exportações para Markdown ou JSON são explícitas.

Evolução sugerida





Captura do OpenCode e Decision Inbox.



Navegação por projeto e decisões confirmadas.



Context Packs manuais para tarefas futuras.



Biblioteca global com PDFs, papers e sites citáveis.



Assistente de engenharia dentro do aplicativo.



API para agentes de implementação consultarem contexto.



Detecção de contradições e reconsideração de decisões.



Novos adapters para outros agentes e fontes.

Limite de autoridade

O sistema pode observar, organizar, recuperar e aconselhar. Ele não deve:





inventar justificativas ausentes;



transformar referências globais em decisões do projeto;



confirmar decisões pelo usuário;



injetar contexto automaticamente antes de provar utilidade;



escrever ou fazer commit no repositório sem ação explícita.

Visão de longo prazo

O resultado final é um ambiente local de inteligência de engenharia: uma combinação de memória decisional por projeto, biblioteca técnica pessoal e assistente capaz de ajudar a tomar decisões contextualizadas e explicáveis.
