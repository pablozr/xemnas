# Sugestões de relações: teste final pela GUI

**Data:** 03/10/2026. **Status:** confirmação e rejeição executadas e auditadas. **Provider:** GPT-6 Luna medium via bridge loopback de avaliação; não é a integração empacotada de produção.

## Corpus e limite de autoria

O clone continua ripgrep no commit 3fce3b5bb0236da2df6d99672afb8a719642eca7. Três documentos foram acrescentados somente ao clone descartável: [substituição do parser](fixtures/xemnas-evaluation-parser-replacement.md), [cache dependente](fixtures/xemnas-evaluation-options-cache.md) e [reversão do parser](fixtures/xemnas-evaluation-parser-reversion.md). Todos declaram ser simulações locais aceitas pelo avaliador, sem decisão dos mantenedores ou código upstream alterado. Servem para exercer operações; não demonstram decisões reais do projeto.

## Extração e confirmação

Atualizar Visão indexou dois documentos novos (5 no total), propôs capturas e executou análises reais. A fixture de cache retornou assessment ok com zero candidatos; a de parser produziu candidato de significância 0,9, confiança 1 e critérios cross_cutting/data_or_contract/constrains_future_work. Não atribuir causa ao zero candidatos sem investigar o descarte. A adoção pela GUI confirmou a escolha e dois vínculos (core/ignore).

O job suggest_relations propôs `supersedes` contra a decisão anterior, copiando uma frase da escolha e justificando a substituição. O coordenador clicou Confirmar na lista do Mapa. O banco registrou outcome=confirmed, criou uma relação supersedes e marcou a decisão anterior como superseded. Não foi criada uma relação antes do clique.

## Rejeição

A fixture de reversão foi indexada (6 documentos) e gerou outro candidato real com significância 0,9. Foi adotado com os dois vínculos. O provider propôs nova substituição, agora da escolha de parser compartilhado. O coordenador clicou Rejeitar. A lista removeu a proposta e exibiu “Relação rejeitada; ela não volta”. A auditoria persistente confirmou outcome=rejected e resolved_at; não criou outra relação e ambas as decisões desse segundo par permaneceram accepted. Isto prova a transição observada; não foi forçada nova execução do finder para comprovar deduplicação após rejeição.

## Auditoria independente do coordenador

O [snapshot final sanitizado](../logs/final-gui-relations-state.json) mostra três decisões (uma superseded, duas accepted), duas propostas de relação (confirmed e rejected), uma relação persistida e 14 jobs completed. `integrity_check=ok`, `foreign_key_check=[]`. As verificações foram por SQL somente leitura; nenhuma transição foi forjada por SQL. As capturas 43/44/47/48 mostram candidato, proposta, rejeição e estado final. Nenhuma política Windows foi alterada.

## O que mudou no parecer

O caminho de descoberta → decisão → entidade → proposta citada → confirmação funcionou com o provider real. Depois de conhecimento arquitetural confirmado, a Visão passou a produzir texto em português e fluxos com entidades (core/ignore e grep-cli), diferentemente da Visão inicial só documental. A navegação também foi exercitada: abrir o fluxo “Interpretar opções de busca” mostrou etapas com entidades; clicar em core levou ao detalhe correto no Mapa, com decisões vigentes e histórico da substituição (capturas 49/50). Isso evidencia valor de enriquecimento a partir de decisões; não prova inferência arquitetural automática do código.

A qualidade ainda exige supervisão: o resumo omite o qualificador “simulação local”, o texto de sugestões de regras ultrapassa botões e regras anteriores continuam coexistindo com decisões novas. A Visão combinou escolha atual e regras anteriores; essa apresentação não substitui resolver escopo, conflito e validade. Os novos dados corrigem o parecer sobre a capacidade operacional do grafo sem apagar as críticas iniciais.

## Limitações e limpeza

O harness e o teste SQLite de sugestões foram bloqueados antes de executar; essa limitação histórica permanece registrada. O caminho GUI acima completou os dois fluxos que faltavam, após o usuário autorizar retomar a tela. Cinco críticas são de agentes do mesmo modelo, não pesquisa com humanos. Não foi demonstrada vantagem de esforço total sobre ADR/manual.

Ao terminar, a janela Xemnas do experimento e os dois processos próprios de bridge/OpenCode foram encerrados. Dados públicos/sintéticos temporários foram preservados para auditoria. Não foram incluídos DB, discovery, runtime, token ou credenciais nos arquivos versionados.
