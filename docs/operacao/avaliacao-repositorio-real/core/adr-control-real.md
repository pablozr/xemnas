# Controle real da decisão: contadores de Override

## Parecer

A exportação registra uma decisão aceita pelo avaliador: preservar a polaridade invertida dos contadores de Override em relação a Gitignore. Não trocar as delegações só para fazer os nomes dos contadores parecerem alinhados.

O motivo registrado é semântico: neste caso, a opção -g inverte a semântica de Gitignore. Um glob sem ! conta como whitelist; um glob com ! conta como ignore. A justificativa também aponta que Override::num_ignores delega a Gitignore::num_whitelists, que Override::num_whitelists delega a Gitignore::num_ignores, e que matched inverte o resultado.

A alternativa rejeitada foi trocar essas delegações para alinhar os nomes dos contadores aos de Gitignore. Isso mudaria a leitura exposta por Override sem respeitar a polaridade descrita para -g.

## O que foi validado

A decisão exportada cita código anexado como evidência da delegação cruzada, da interpretação de globs com e sem ! e da inversão de matched. Ela também registra a existência de um teste chamado counts, mas declara explicitamente que nenhum comando foi executado. Portanto, não há validação registrada de compilação nem execução do teste; não apresento isso como resultado confirmado.

## Autoridade e escopo

É uma decisão local do avaliador, não orientação dos mantenedores do ripgrep. O JSON deixa scope, ssumptions, econsider_when e consequences vazios. Assim, esta exportação não define um escopo adicional nem critérios formais para reconsiderar a decisão.

## Esforço mecânico e limites deste controle

Este controle consultou somente decision-export-gui.json: uma leitura, com tamanho de entrada de 2908 bytes, e uma gravação deste Markdown. Não examinei código, artefatos de evidência, banco de dados, interface nem outros relatórios. Não medi diferenças de captura ou de contexto automático, e não afirmo que sejam equivalentes ou irrelevantes.

Como registro curto da decisão e de seus limites de validação, o documento é suficiente para retomar a escolha sem reapresentar a avaliação como teste executado. Não substitui a evidência técnica original nem uma validação futura, caso ela seja necessária. Também não permite inferir economia de tempo humano ou preferência baseada em telas que não foram vistas.
