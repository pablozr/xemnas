# Decisões — variante C escolhida

Em 29/09/2026, o usuário escolheu C para diferenciar a leitura de decisões da Revisão e pediu um acabamento próximo do Linear. Esta escolha substitui a recomendação inicial por A; o estudo anterior continua disponível para comparação.

## Direção aplicada

- Documento no centro e índice temporal à direita. Navegação de projetos discreta, com seleção persistente e lavanda mineral reservada a estado/foco/ação.
- Grafite neutro e menos saturado; barra de localização contínua; abas compactas; ferramentas em lugares estáveis; divisores suaves. Referência primária: [Linear, A calmer interface for a product in motion, 12/03/2026](https://linear.app/now/behind-the-latest-design-refresh). A composição e os valores abaixo são decisões para Xemnas, não medidas prescritas pelo Linear.
- Pergunta em 28 px, peso 500; escolha confirmada em 18 px com traço lateral; justificativa em prosa; contexto em duas colunas, com expansão sob demanda. O texto principal tem largura máxima de 740 px.
- Em 1440 px: projetos 232 px, índice 296 px, documento ocupa o restante. Em 1180 px: projetos 220 px, índice 248 px, título 25 px e escolha 16 px. Esta é uma proposta de compactação para a nova tela, diferente das medidas iniciais do estudo A.
- Documento/Histórico e Revisar/Exportar ficam no cabeçalho fixo da leitura. O índice conserva busca e filtro ao rolar a coleção; a seleção tem superfície e traço próprios, preservados sob hover.
- Fontes com ícone SVG, abas anexadas, caminho registrado, intervalo de linhas, copiar conteúdo e leitura ampliada. Prosa tem quebra de linha; código preserva whitespace e rolagem própria. Copiar usa só o conteúdo exibido, sem números de linha ou metadados acrescentados.
- Badges conservam texto e bolinha de cor. Quantidade no índice significa itens carregados/resultados deste estudo, não total do banco.

## Arquivos e validação

[Abrir C refinada](http://127.0.0.1:8769/prototype-decisions.html?variant=C). Para comparar as alternativas, adicionar `&compare=1`; o seletor é oculto na apresentação da opção escolhida.

Arquivo principal: [prototype-decisions.html](prototype-decisions.html); detalhes de C: [CSS](prototype-decisions-c.css) e [composição/interações](prototype-decisions-c.js). O material anterior foi preservado em [prototype-decisions-before-c-polish.html](prototype-decisions-before-c-polish.html).

Conferidos no navegador: documento/índice à direita, fonte ampliada, troca de arquivo, histórico de versão anterior somente leitura, preview de exportação da versão atual, busca e vazio, formulário/cancelamento, viewport de 1180 × 760 sem overflow horizontal. Nenhum erro JavaScript. [Roteiro](prototype-decisions-c-qa.js).

[Visão padrão](decisions-c-polished.png), [compacta](decisions-c-compact.png), [histórico](decisions-c-history.png), [fonte ampliada](decisions-c-source-expanded.png), [busca sem resultado](decisions-c-empty.png).

Este estudo usa dados fictícios e não persiste alterações nem exporta arquivos. Ainda não é a tela GPUI. Na implementação, cores devem entrar nos tokens do projeto, dados vir dos ports existentes e corpos de evidência ser resolvidos no caso de uso. Permanecem as lacunas registradas na [pesquisa do backend](research-next-screen.md): contagem total, busca com cursor/ranking e exportação de versões antigas não devem ser simuladas como funções reais.
