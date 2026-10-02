# Vinte ideias adicionais para o Xemnas, a partir de dores de desenvolvedores

**Data:** 02/10/2026.
**Pergunta:** que vinte utilidades adicionais podem ampliar o valor do Xemnas,
inspiradas em evidência científica e trabalho real, além da comparação de mercado?
**Status:** Aberta. Pesquisa concluída; ideias exploratórias, não implementadas,
sem validação de demanda ou eficácia no Xemnas. Não alteram o escopo aprovado.

## Direção recomendada

Além de preservar escolhas, o Xemnas pode preservar **o conhecimento necessário
para investigar, entender e verificar uma mudança**. A oportunidade é ajudar
quando o desenvolvedor precisa responder “o que já sabemos?”, “o que ainda não
testamos?” e “como conferir isso sem depender da narrativa do agente?”.

As vinte ideias abaixo são adicionais às [dez propostas anteriores](oportunidades-produto-memoria-decisional.md).
Há componentes compartilhados, mas cada ideia possui uma situação de uso e uma
entrega próprias. Não recomendar construir vinte telas ou vinte produtos separados.

Minhas primeiras apostas são **Quadro de hipóteses**, **Dossiê de reprodução**,
**Atlas de exemplos internos**, **Intenção antes do diff** e **Mapa de verificação**.
Constituem dois percursos: investigar sem recomeçar e revisar entendendo o que
precisa ser verdadeiro. São prioridades qualitativas, ainda sem demanda comprovada.

## Base científica e método

32 entradas de fontes primárias foram organizadas em quatro catálogos:

| Chave usada nesta nota | Catálogo | Evidência |
| --- | --- | --- |
| CG, C1–C8 | [Cognição e investigação](dores-dev-cognicao-fontes.md) | Campo, logs, observação, surveys e experimentos de compreensão/debugging |
| EQ, E1–E8 | [Conhecimento de equipe](dores-dev-conhecimento-equipe-fontes.md) | Onboarding, code review, concentração de autoria, dívida e coordenação |
| IA, IA1–IA8 | [IA e verificação](dores-dev-ia-verificacao-fontes.md) | Experimentos, surveys, segurança, compreensão e delegação |
| CT, C01–C08 | [Contratos e reprodução](dores-dev-contratos-reproducao-fontes.md) | Linguagem, APIs, requisitos, reprodução de bugs e adoção de ferramentas |

Entradas não equivalem a 32 experimentos independentes: há atualização de um
estudo e diferentes métodos. População, versão, amostra, limites e links originais
estão nos catálogos. Parte da leitura foi de resumo institucional/abstract, com
acesso parcial identificado. Não houve consulta exaustiva a bases pagas nem revisão
sistemática pré-registrada, metanálise, entrevistas próprias ou instalação de produtos.

Os estudos fundamentam dores nos contextos investigados. **Os desenhos de produto
são inferências nossas**, não conclusões dos autores. Dor não prova compra;
evidência de produtividade num experimento não mede o valor do Xemnas.

### O que a evidência muda na escolha

- Compreender e investigar são parte necessária do trabalho. Tempo lendo código
  não pode ser contado automaticamente como desperdício (CG/C5).
- Pessoas precisam de intenção e comportamento esperado, além de símbolos; há
  conhecimento que fica inacessível quando o colega não está disponível (CG/C3).
- Code review envolve entendimento da mudança e transferência de conhecimento
  (EQ/E2). Contar comentários não demonstra melhoria.
- A comparação de IA deve considerar tarefa, experiência, qualidade e atenção:
  estudos de campo favoráveis e o METR medem coisas/populações diferentes;
  atualização METR 2026 tem problemas de seleção (IA/IA1–IA3).
- Código pronto, confiança e compreensão são medidas distintas; estudos de
  segurança/aprendizagem oferecem motivos para verificar, sem concluir que todo
  código gerado hoje é inseguro ou que toda assistência prejudica aprendizagem
  (IA/IA4–IA5).
- Uma previsão por commits sobre conhecimento e um aviso por palavras sobre
  requisitos têm limites. Testar entendimento e mostrar evidência é mais seguro
  que um score opaco (EQ/E3; CT/C07).

## As vinte ideias

Todos os testes abaixo são **experimentos propostos**, não resultados. Podem
começar com artefatos locais curados, antes de contratar serviços ou construir
integrações. Esforço é relativo; novos registros persistentes precisam de contrato.

### 1. Quadro de hipóteses de debugging

**Dor/cena:** “é cache ou concorrência?”; humano e agente mudam de explicação sem
guardar a observação que já contrariou a anterior.
**Experiência:** fichas de hipótese → previsão → teste → resultado, com estados
aberta, refutada ou inconclusiva. Exemplo: “se for cache, limpar X deve mudar Y”.
**Base:** CG/C6 e C8; CT/C03. **Novidade local:** raciocínio temporário sobre uma
falha, diferente de premissa durável de uma decisão.
**Piloto:** seis bugs reais; comparar repetição dispensável, qualidade da conclusão
e minutos para manter fichas. **Limite:** hipótese gerada pode ancorar num erro;
não sugerir probabilidade inventada nem transformar ficha em decisão confirmada.
**Esforço:** médio; versão manual pequena. **Prioridade:** alta.

### 2. Memória dos caminhos que não explicaram o bug

**Dor/cena:** o próximo agente repete uma busca e um teste já inconclusivos.
**Experiência:** recuperar pergunta, caminho investigado, versão/configuração,
resultado e motivo para parar. Exibir “não explicou neste ambiente”, com possibilidade
de reabrir. **Base:** CG/C4 e C8.
**Novidade local:** conhecimento negativo da investigação, diferente de tecnologia
descartada numa escolha arquitetural.
**Piloto:** três handoffs de bugs; distinguir repetição evitável de repetição útil
por mudança de versão. **Limite:** ausência de resultado não prova ausência da causa.
Retenção curta e curadoria evitam acumular buscas sem valor. **Esforço:** médio;
compartilha a base da ideia 1, mas testa a transferência, não a formulação da hipótese.

### 3. Dossiê de reprodução transferível

**Dor/cena:** “na minha máquina funciona”; ninguém consegue reconstruir o estado
que produziu a falha. **Experiência:** pacote selecionado com revisão Git, versões,
entrada mínima sanitizada, passos, esperado/observado e evidência. Outra pessoa
anexa sua tentativa sem sobrescrever a primeira.
**Base:** CT/C03–C05; CG/C8. **Novidade local:** preservar condições específicas
da reprodução; não é retomada genérica ou relatório de resultado de decisão.
**Piloto:** cinco bugs transferidos a outro desenvolvedor; sucesso de reprodução,
perguntas extras e custo de preparar o pacote. **Limite:** não gravar ambiente inteiro,
segredos ou rodar scripts recebidos. Comandos são dados; execução inicial manual.
**Esforço:** médio. **Prioridade:** alta.

### 4. Mapa de comportamento para código

**Dor/cena:** issue diz “confirmar duplica a decisão”, mas não cita nenhum símbolo.
**Experiência:** selecionar a ação reconhecível e encontrar entrada, caso de uso,
teste e decisões associadas. Mostrar se o caminho foi registrado por humano,
exercitado em teste ou apenas sugerido.
**Base:** CG/C3–C4. **Novidade local:** navegação pelo comportamento do produto,
com evidência; diferente de selecionar um arquivo ou renderizar todo o grafo.
**Piloto:** curar cinco ações e pedir a um dev localizar onde investigar;
medir pontos de entrada corretos e erros de orientação. **Limite:** associação
estática não comprova execução e o mapa pode ser incompleto. **Esforço:** médio;
começar por links manuais, sem construir analisador multilinguagem.

### 5. Glossário que conecta domínio, usuários e símbolos

**Dor/cena:** “revisão” significa inbox, review de PR e análise consultiva em
conversas diferentes. **Experiência:** conceito confirmado, termos de usuários,
homônimos, exemplos e símbolos relacionados; busca explica qual sentido recuperou.
**Base:** CT/C01; CG/C3–C4.
**Novidade local:** vocabulário navegável de cada Project, ligado à evidência;
o CONTEXT já existente continua sendo a regra do próprio Xemnas.
**Piloto:** dez consultas ambíguas antes/depois; relevância e confusão entre conceitos.
**Limite:** IA sugere equivalências, pessoa confirma. Similaridade de palavras não
autoriza unificar entidades ou fazer rename. **Esforço:** baixo/médio para curadoria;
alto se tentar inferir automaticamente toda a linguagem de domínio.

### 6. Atlas de exemplos internos com limites de uso

**Dor/cena:** agente copia um padrão que parecia certo, mas era uma exceção.
**Experiência:** exemplos curados de “como fazemos aqui”, cada um com intenção,
commit, decisão que ilustra, quando usar e quando não copiar. Exemplo: um retry
aceito para operação idempotente não serve para repetir cobrança.
**Base:** CG/C4; IA/IA6. **Novidade local:** precedentes do código do Project,
diferente da biblioteca global de livros ou memória de alternativas rejeitadas.
**Piloto:** cinco exemplos em novas tarefas; avaliar aplicação adequada e cópias
indevidas. **Limite:** exemplo continua sendo evidência e pode envelhecer; não
ganha autoridade só por estar no atlas. **Esforço:** baixo/médio, curadoria manual.
**Prioridade:** alta.

### 7. Dossiê de migração de dependência

**Dor/cena:** o upgrade compila, mas muda a semântica em que o projeto confiava.
**Experiência:** versões antiga/nova, contrato usado, referências oficiais, pontos
conhecidos de uso e testes que verificam essas necessidades. O usuário registra
o que permanece desconhecido antes de autorizar a migração.
**Base:** CT/C02; IA/IA6. **Novidade local:** conferir uma dependência concreta;
diferente de ensaiar uma substituição arquitetural em todo o grafo.
**Piloto:** três upgrades reais; comparar perguntas descobertas antes da integração
e retrabalho. **Limite:** referência deve corresponder à versão instalada; o estudo
Java não comprova viabilidade para Rust ou GPUI. Não prometer compatibilidade completa.
**Esforço:** médio com referências manuais; alto para análise automática.

### 8. Intenção antes do diff

**Dor/cena:** reviewer recebe vinte arquivos sem entender o comportamento que
precisa mudar ou continuar igual. **Experiência:** cartão curto com problema,
objetivo, limites, decisões aplicáveis e evidências anexadas pelo autor, exportável
para o review. Declaração de intenção e implementação observada aparecem separadas.
**Base:** EQ/E2. **Novidade local:** preparação para compreensão humana;
diferente do recibo do que foi enviado ao agente.
**Piloto:** dez reviews comparáveis; perguntas de esclarecimento, entendimento e
tempo de preparar/ler o cartão. **Limite:** resumo plausível pode esconder divergência;
conservar acesso ao diff original e exigir revisão do autor. **Esforço:** baixo/médio
com exportação explícita. **Prioridade:** alta.

### 9. Diff organizado por intenção

**Dor/cena:** renomeação, mudança de regra e ajuste de teste estão misturados.
**Experiência:** grupos sugeridos por objetivo, ligados às decisões e com arquivos
compartilhados visíveis. A pessoa corrige agrupamentos; cada linha continua acessível
na alteração original.
**Base:** EQ/E2; IA/IA6 e IA8. **Novidade local:** organizar uma alteração real,
diferente de prever impacto de uma mudança hipotética; complementa, não substitui,
o cartão da ideia 8.
**Piloto:** cinco diffs mistos, comparados à ordem por arquivo; entendimento correto
e detecção de mudança fora do objetivo. **Limite:** grupos podem esconder interação;
indicar cobertura e itens não classificados. **Esforço:** alto para integração/diff;
primeiro validar agrupamento manual.

### 10. Ensaio de transferência de conhecimento

**Dor/cena:** só uma pessoa consegue explicar um módulo; o histórico de commits
não diz se outra pessoa o entende. **Experiência:** o mantenedor escolhe cinco
perguntas reais; outro dev responde usando a memória; lacunas viram conversa ou
pairing escolhido por ambos.
**Base:** EQ/E3 e E7; CG/C3. **Novidade local:** exercício aplicado de transferência,
sem estimar bus factor nem medir competência por autoria.
**Piloto:** duas transferências; acerto das respostas, fontes encontradas e custo
de complementar conhecimento. **Limite:** não virar avaliação de funcionário ou
plano automático de substituição. Participação e compartilhamento voluntários.
**Esforço:** baixo/médio em pacote local; colaboração multiusuário seria extensão.

### 11. Passaporte de interface

**Dor/cena:** backend e front entendem versões diferentes de “cancelar” ou “erro”.
**Experiência:** acordo da fronteira com produtor, consumidores conhecidos,
invariantes, exemplos aceitos e itens que exigem consulta. Cada participante confirma
o entendimento; pode começar como exportação Markdown.
**Base:** EQ/E5; CT/C02. **Novidade local:** preparar coordenação numa interface
compartilhada, diferente de isolar branches ou simular impactos.
**Piloto:** cinco mudanças de contrato; diferenças de entendimento descobertas
antes da integração e perguntas de esclarecimento. **Limite:** consumidores conhecidos
não significam todos os consumidores. Coalteração não prova obrigação de contatar
alguém. Não enviar mensagens automaticamente. **Esforço:** médio manual; alto para
sincronização de equipe. Bom caso para o próprio fluxo backend/front do Xemnas.

### 12. Mesa de critérios entre pessoas

**Dor/cena:** uma discussão parece disputa de tecnologias, mas as pessoas estão
otimizando objetivos diferentes. **Experiência:** três critérios por participante,
cenários observáveis e restrições; quadro evidencia desacordos de prioridade e uma
pergunta/experimento que ajudaria a resolvê-los.
**Base:** EQ/E6 e E8. **Novidade local:** negociar objetivos entre pessoas,
diferente de recuperar opções já descartadas ou recomendar sozinho uma arquitetura.
**Piloto:** cinco discussões; cada pessoa consegue reproduzir os critérios da outra
e identificar o acordo feito? **Limite:** não fabricar consenso, atribuir pensamento,
rankear cargos ou gerar score vencedor. **Esforço:** baixo/médio num encontro
facilitado; não exige colaboração cloud para experimentar.

### 13. Primeira tarefa com explicação de volta

**Dor/cena:** o recém-chegado leu docs, mas não consegue aplicar as restrições.
**Experiência:** tarefa pequena escolhida pelo mantenedor, três decisões/arquivos
de referência e uma explicação voluntária de como a mudança os respeita. O mantenedor
valida a compreensão e conversa sobre o que faltou.
**Base:** EQ/E1 e E7. **Novidade local:** onboarding aplicado de quem ainda não
conhece o projeto, diferente de retomar depois de uma pausa.
**Piloto:** cinco primeiras tarefas; compreensão predefinida, dúvidas e tempo total.
**Limite:** não substituir mentoria por quiz nem registrar nota pessoal.
**Esforço:** baixo/médio com curadoria; benefícios de equipe dependem de compartilhar
o pacote, recurso que deve ser autorizado e projetado separadamente.

### 14. Caderno de fricção da dívida técnica

**Dor/cena:** “isso atrapalha sempre” não basta para negociar uma melhoria.
**Experiência:** durante uma tarefa, anotação pequena do obstáculo, workaround,
tempo aproximado informado e evidência; relacionar ao atalho/compromisso conhecido.
Um conjunto de episódios ajuda a explicar o custo real de manter a escolha.
**Base:** EQ/E4; CT/C08. **Novidade local:** fundamentar priorização da dívida
com trabalho observado, não acompanhar genericamente o resultado de toda decisão.
**Piloto:** duas semanas, uma área; verificar se relatos apoiam uma priorização e
se registrar custa menos do que ajuda. **Limite:** relato não prova causa exclusiva;
não calcular juros fictícios, somar estimativas como horas auditadas ou vigiar devs.
**Esforço:** baixo/médio manual.

### 15. Critérios de comportamento antes de gerar código

**Dor/cena:** “faça cache” deixa TTL, consistência e erro como escolhas silenciosas.
**Experiência:** poucos exemplos de entrada/saída, bordas e efeitos proibidos,
confirmados pelo humano, exportados junto à tarefa. Exemplo: “mesma chave com usuário
diferente nunca reutiliza dado privado”.
**Base:** CT/C07; IA/IA4 e IA6. **Novidade local:** especificar uma tarefa nova,
diferente de perguntar sobre lacunas na memória de uma decisão existente.
**Piloto:** dez tarefas vagas; decisões implícitas indevidas e retrabalho, considerando
custo de preparação. **Limite:** sugestões são critérios propostos, não requisitos
verdadeiros; exemplo não prova cobertura completa. **Esforço:** baixo/médio manual;
o produto não precisa executar nem escrever testes automaticamente.

### 16. Envelope de delegação por tarefa

**Dor/cena:** protótipo e migração de dados não têm o mesmo limite de autonomia.
**Experiência:** briefing com objetivo, superfícies permitidas, escolhas reservadas
ao humano e condições de parada. Recuperável por contexto/MCP ou exportação.
**Base:** IA/IA7–IA8. **Novidade local:** compromisso específico daquela tarefa,
diferente de configurações globais de injeção ou contexto por branch.
**Piloto:** dez tarefas; desvios concretos e custo de preparar o envelope.
**Limite:** Xemnas fornece orientação; bloqueio coercitivo depende do executor.
Não exibir um escudo de proteção se o adapter não impõe o limite. Autorizar
leitura não autoriza escrita, deploy ou alteração de dados. **Esforço:** baixo/médio
como artefato; alto para enforcement integrado.

### 17. Mapa de verificação: obrigação → evidência

**Dor/cena:** agente diz “terminado”, mas testes verdes não verificam rollback ou
compatibilidade. **Experiência:** cada critério humano aponta teste/inspeção e revisão
do código correspondentes, ou fica explicitamente não verificado. Resultados anexados
por pessoa/ferramenta autorizada, separados da explicação do gerador.
**Base:** IA/IA1 e IA4; CT/C07. **Novidade local:** verificar o resultado da
alteração, diferente do recibo de contexto e de um laboratório A/B de prompts.
**Piloto:** dez mudanças com critérios conhecidos; omissões encontradas e tempo
de revisão. **Limite:** evidência pode ser insuficiente; checklist completo não
certifica segurança/correção. Não executar código desconhecido automaticamente.
**Esforço:** médio manual, alto integrado. **Prioridade:** alta.

### 18. Compreender a mudança antes de aceitá-la

**Dor/cena:** pessoa aceita código novo, mas não sabe prever o caso de erro.
**Experiência:** leitura opcional antes/depois, link a trechos e oportunidade de
prever uma saída antes de revelar a explicação; conferir um comportamento importante
com evidência e permitir pergunta conceitual.
**Base:** IA/IA5–IA6. **Novidade local:** supervisão de uma alteração produzida
com IA, diferente de onboarding de alguém novo no projeto ou retomada de tarefa.
**Piloto:** tarefas com biblioteca desconhecida; compreensão imediata e correção
de defeito posterior, além de tempo. **Limite:** explicação pode estar errada;
nunca quiz obrigatório, perfil psicológico ou score de funcionário.
**Esforço:** médio; usar exemplos e fontes humanos antes de explicação automática.

### 19. Cartões de erros que viraram entendimento

**Dor/cena:** após descobrir que cancelamento não desfaz gravação, a lição some.
**Experiência:** salvar voluntariamente interpretação inicial, contraexemplo,
explicação corrigida e referência; recuperar quando o conceito reaparece.
**Base:** IA/IA5–IA6. **Novidade local:** corrigir modelo mental e transferir a
lição a outra tarefa; não registrar uma alternativa tecnológica rejeitada ou só
o resultado de uma decisão.
**Piloto:** cinco cartões e tarefa posterior, inclusive em outra semana;
compreensão/aplicação correta e cartões dispensados. **Limite:** estudo de quiz
imediato não prova retenção prolongada; diagnóstico pode ser revogado. Cartão não
vira regra automaticamente. **Esforço:** baixo/médio; sem feed diário ou gamificação.

### 20. Memória da triagem de avisos

**Dor/cena:** o mesmo warning volta; ninguém sabe por que foi corrigido,
dispensado ou aceito com condição. **Experiência:** importar/anexar aviso escolhido,
registrar decisão de triagem, evidência, versão e escopo; recuperar caso comparável
na revisão seguinte, sem alterar configuração do scanner.
**Base:** CT/C06. **Novidade local:** aprendizado da triagem de ferramentas,
diferente de radar de premissas ou revisão que procura violações no código.
**Piloto:** vinte avisos reais; repetição de investigação, dispensas indevidas
e custo de manter o motivo. **Limite:** dispensa antiga não prova falso positivo
atual; ignorar um aviso não confirma segurança. Não silenciar alertas nem copiar
supressões para outros escopos. **Esforço:** médio; começar com registros manuais.

## Comparação e prioridade

Estimativas qualitativas de proximidade ao produto e dependências; não são scores
calculados, preços, prazos nem comprovação de diferencial de mercado.

| Ideia | Quem sente primeiro | Primeiro artefato | Dependência maior | Escolha |
| --- | --- | --- | --- | --- |
| 1 Hipóteses | Dev/agent em debugging | Fichas com testes | Recuperação por investigação | Piloto principal |
| 2 Caminhos sem resultado | Próximo investigador | Trilhas contextualizadas | Retenção e comparação de ambiente | Junto ao 1, depois de medir |
| 3 Reprodução | Quem recebe um bug | Pacote sanitizado | Ambiente/artefatos seguros | Piloto principal |
| 4 Comportamento→código | Quem conhece a dor, não símbolos | Cinco percursos curados | Manutenção dos vínculos | Piloto restrito |
| 5 Glossário | Pessoa/agente com termos ambíguos | Dez conceitos | Confirmação de equivalências | Base útil |
| 6 Atlas | Implementador usando precedentes | Cinco exemplos | Validade e escopo | Piloto principal |
| 7 Migração | Dev atualizando dependência | Dossiê de versão | Cobertura de uso real | Quando surgir upgrade |
| 8 Intenção no review | Autor e reviewer | Cartão exportável | Adapter para mudança | Piloto principal |
| 9 Diff por intenção | Reviewer | Agrupamento manual | UI/diff e cobertura | Depois do 8 |
| 10 Transferência | Mantenedor/equipe | Cinco perguntas | Compartilhamento consentido | Validar equipe primeiro |
| 11 Interface | Backend/front | Acordo curto | Coordenação compartilhada | Dogfood do Xemnas |
| 12 Critérios | Pessoas decidindo juntas | Quadro de prioridades | Participação humana | Experimento facilitado |
| 13 Primeira tarefa | Recém-chegado | Tarefa com explicação | Mentoria | Validar equipe primeiro |
| 14 Fricção da dívida | Dev e quem prioriza | Episódios curtos | Atribuição/custo de registrar | Piloto de duas semanas |
| 15 Comportamento | Autor de uma tarefa | Exemplos aceitos/rejeitados | Clareza do objetivo | Habilitador do 17 |
| 16 Delegação | Pessoa com vários agentes | Envelope explícito | Enforcement do executor | Manual primeiro |
| 17 Verificação | Quem aprova resultado | Obrigações e evidências | Resultados/versões exatos | Piloto principal |
| 18 Compreensão | Quem aceita código desconhecido | Predição + leitura | Qualidade da explicação | Medir aprendizagem |
| 19 Erro entendido | Quem aprendeu algo difícil | Cartão de contraexemplo | Recuperação pertinente | Pequeno experimento |
| 20 Triagem | Dev sobrecarregado de warnings | Aviso + motivo | Escopo/versionamento | Se houver repetição real |

**Pacote inicial A — investigação compartilhável:** 1 + 3, com 2 se houver
repetição comprovada. **Pacote inicial B — revisão compreensível:** 8 + 15 + 17.
**Base de reutilização:** 5 + 6. Escolher um pacote para o primeiro dogfood, sem
começar integrando todas as ideias ao mesmo tempo.

## Como não repetir as propostas anteriores

- Radar pergunta se uma escolha continua válida; 1–3 organizam uma falha em
  investigação e não confirmam decisão durável.
- Ensaio de mudança avalia cenário hipotético; 8–9 apresentam alteração real.
- Retomada restaura contexto para quem volta; 13 trata primeira entrada e 18
  trata compreensão do que se está aprovando agora.
- Biblioteca aplicada usa fontes externas; 6 cura exemplos internos de código.
- Recibo explica contexto entregue; 17 explica evidência do resultado produzido.
- Alternativas preservam escolhas descartadas; 2 preserva buscas/testes e 19
  preserva uma interpretação corrigida.
- Perguntas que faltam enriquecem a memória; 15 define aceitação da tarefa nova.
- Resultado de decisão aproxima expectativa e observação; 14 registra fricção
  durante o trabalho para apoiar negociação de dívida, sem afirmar causa única.

São distinções de utilidade. Na implementação, funcionalidades próximas podem
compartilhar dados e uma superfície; a separação aqui serve para avaliar cada dor.

## Restrições de produto e arquitetura

A base atual fornece Source Artifacts, decisões, evidências, versões e Context
Packs. **Não foi demonstrado que suporte os novos registros e fluxos sem extensão.**
Investigação temporária não deve inundar a Decision Inbox ou virar regra durável;
a seleção de memória precisa distinguir hipótese, observação e decisão confirmada.

O [ADR-0009](../arquitetura/adr/0009-revisao-consultiva-de-conhecimento.md) continua
limitando revisão a consulta efêmera, sob demanda, sem jobs/escritas ou detecção de
violações no código. Persistir hipóteses, análise de diff, resultado de teste e
colaboração requer casos de uso e contratos próprios. Não esconder isso na UI.

Núcleo local, máquina de 8 GB/CPU e timeout de contexto do adapter continuam
restrições; análises profundas não devem entrar automaticamente em cada turno.
Nenhuma ideia exige renderizar o grafo completo ou armazenar tela/clipboard.
Execução, escrita no repositório, compartilhamento e destinos de inferência precisam
ser escolhas concretas do usuário. Não ampliar retenção de prompts silenciosamente.

Ao implementar, atualizar documentação duradoura/ADR e remover esta pesquisa e
fontes exclusivas conforme [as regras da pasta](README.md). Nenhuma nova tela foi
desenhada ou alterada nesta etapa.

## Protocolo de validação proposto

1. Entrevistar 6–8 devs: independentes, manutenção de legado, equipe backend/front
   e uso de agentes. Pedir o último episódio real, evidência, workaround e custo;
   não começar apresentando a lista de recursos.
2. Escolher duas dores recorrentes e preparar artefatos manuais com dados
   sanitizados. Comparar com o fluxo habitual, inclusive notas/README bem escritos.
3. Usar tarefas distintas, ordem alternada e critérios fixados antes do piloto;
   evitar confundir familiaridade adquirida com efeito da ferramenta.
4. Medir resultado específico: reprodução, compreensão correta, repetições
   dispensáveis, obrigações não verificadas e perguntas de esclarecimento. Medir
   também minutos de curadoria, omissões, erros novos e informação sensível removida.
5. Reportar falhas/desistências e denominadores. Tempo menor com mais erro é pior;
   confiança subjetiva e quantidade de cartões não demonstram valor.
6. Voltar ao recurso numa tarefa posterior. Abandonar se a nota simples funciona
   melhor, se a curadoria custa mais do que ajuda ou se a pessoa não recupera o
   conhecimento sem lembrança do pesquisador.

O piloto não mede causalidade em grande escala ou disposição a pagar. Um próximo
estudo pode ampliar amostra e controlar confundidores; primeiro precisamos provar
que o percurso pequeno ajuda a resolver uma dor real.

## Limitações e entrega

Os quatro catálogos registram versões, populações e acessos parciais. Pesquisa
exploratória, sem promessa de novidade mundial, estatística universal ou retorno
financeiro. Nenhuma funcionalidade, benchmark de produto ou entrevista foi executado.
Esta entrega é a base concreta para escolher experimentos, não aprovação de roadmap.
