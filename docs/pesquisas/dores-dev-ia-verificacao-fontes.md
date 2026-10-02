# Dores de desenvolvimento com IA: verificação, compreensão e autonomia

**Data:** 02/10/2026.
**Pergunta:** que dores empiricamente observadas no uso de IA por desenvolvedores
podem orientar novas oportunidades para o Xemnas?
**Status:** Aberta. Pesquisa exploratória; propostas sem validação de demanda ou
eficácia no Xemnas. Não altera o escopo aprovado.

## Método e cuidado com as inferências

Busca e abertura de pesquisas primárias em arXiv, páginas dos autores e instituições
de pesquisa. Foram priorizados experimentos, observação de trabalho e surveys com
população descrita, combinando resultados favoráveis e desfavoráveis à assistência
por IA. Não é revisão sistemática: não houve protocolo pré-registrado, dupla seleção,
meta-análise ou busca exaustiva em bases pagas.

Evidência de dor não comprova demanda por uma funcionalidade. Estudos antigos de
Copilot/Codex não medem agentes de outubro de 2026. Surveys de preferência não
provam efeito causal; experimentos curtos de aprendizagem não provam perda de
competência ao longo de anos. As propostas abaixo são inferências nossas.

## Fontes primárias

### IA1 — METR: percepção de velocidade pode divergir do trabalho concluído

[Becker, Rush, Barnes e Rein, estudo de julho de 2025](https://metr.org/blog/2025-07-10-early-2025-ai-experienced-os-dev-study/).

RCT com 16 contribuidores experientes, 246 tarefas reais em repositórios maduros
conhecidos pelos participantes. Cada tarefa foi sorteada para permitir ou impedir
IA; houve gravação da tela e tempo informado. Predominaram Cursor e Claude
3.5/3.7. Com IA, o tempo aumentou 19%; participantes ainda estimaram ganho de
20% após o trabalho. A população pequena, familiaridade prévia e período dos
modelos impedem generalização para todo desenvolvimento. O resultado não prova
que IA seja inútil. Dor relevante: produzir sugestões rapidamente pode deslocar
o trabalho para interpretação, correção e integração, sem que a sensação de
fluidez capture esse custo.

### IA2 — METR 2026: medir trabalho com agentes paralelos é difícil

[Atualização de 24/02/2026](https://metr.org/blog/2026-02-24-uplift-update/).

Seguimento com 57 desenvolvedores, 143 repositórios e mais de 800 tarefas;
10 participantes vieram do estudo anterior. As estimativas brutas passaram
a sugerir aceleração, mas os autores consideram o sinal não confiável: recusa
em trabalhar sem IA, seleção de tarefas, menor remuneração e dificuldade de
atribuir tempo quando há agentes concorrentes. As estimativas têm intervalos
que incluem ausência de efeito. Portanto, não apresentar o número de 2025
como diagnóstico atual universal, nem o seguimento como comprovação de ganho.
Dor relevante: tempo de relógio, atenção humana, qualidade e trabalho paralelo
são coisas diferentes; uma métrica única de produtividade pode enganar.

### IA3 — Cui e colegas: assistência também pode aumentar entregas

[Três experimentos de campo, página dos autores na Microsoft Research](https://www.microsoft.com/en-us/research/publication/the-effects-of-generative-ai-on-high-skilled-work-evidence-from-three-field-experiments-with-software-developers/).

Experimentos randomizados nas operações de Microsoft, Accenture e uma empresa
Fortune 100; 4.867 desenvolvedores receberam ou não acesso a sugestões de código
por IA. Na análise combinada, os autores estimaram 26,08% mais tarefas concluídas
(erro padrão 10,3%), com maior adoção e benefício para menos experientes. Cada
experimento é ruidoso. A página primária resume o estudo; não auditamos os dados
brutos. Tarefas concluídas e tempo por issue de contribuidores experientes não
são a mesma medida: este resultado não cancela o METR. Dor/oportunidade:
suporte precisa variar por tarefa, experiência e custo de integração; impedir
IA indiscriminadamente também pode remover benefícios.

### IA4 — Perry e colegas: confiança excessiva em segurança

[CCS 2023, artigo completo](https://arxiv.org/html/2211.03622v3).

Estudo com 47 participantes e cinco tarefas de segurança em Python, JavaScript
e C, comparando acesso a um assistente baseado em codex-davinci-002 com controle.
No conjunto, o grupo assistido escreveu código menos seguro e mostrou maior
crença de que suas soluções eram seguras. Há variação por tarefa e participante;
o modelo é antigo, tarefas delimitadas e amostra não representam toda engenharia.
Não autoriza afirmar que código gerado hoje é sempre menos seguro. Dor relevante:
aparência plausível e confiança subjetiva podem substituir evidência técnica.
O Xemnas pode registrar obrigações e provas de verificação, sem certificar
segurança ou se apresentar como scanner de vulnerabilidades.

### IA5 — Shen e Tamkin: terminar uma tarefa não significa compreender o código

[How AI Impacts Skill Formation, versão 2](https://arxiv.org/html/2601.20245v2);
[relato dos autores](https://www.anthropic.com/research/AI-assistance-coding-skills).

RCT principal com 52 participantes, 26 por condição, usando Python regularmente
há mais de um ano, familiarizados com IA e sem experiência em Trio. Duas tarefas
com essa biblioteca e quiz imediato: médias 50% com IA versus 67% sem IA,
diferença de 17 pontos percentuais; ganho de tempo não significativo. Padrões
de uso com perguntas conceituais se associaram a maior compreensão, sem prova
causal entre esses padrões e aprendizado. Não mede retenção prolongada. O blog
descreve maioria junior, mas a tabela do artigo informa maioria com sete ou mais
anos de programação; usamos os critérios e a tabela, sem equiparar novidade na
biblioteca a senioridade. Dor: dificuldade de supervisionar o que se conseguiu
produzir, sobretudo ao aprender APIs/conceitos novos.

### IA6 — Grounded Copilot: exploração requer outro tipo de suporte

[Barke, James e Polikarpova, PACMPL/OOPSLA 2023](https://arxiv.org/html/2206.15000v3).

Observação qualitativa com 20 participantes, 15 da academia e cinco da indústria,
em tarefas de Python, Rust, Haskell e Java; nove já usavam Copilot. A análise
de teoria fundamentada distinguiu aceleração de uma intenção conhecida e
exploração de algo incerto. Sugestões longas podiam interromper o fluxo; exploração
envolvia validação explícita por execução, documentação e ferramentas. Não é
estimativa populacional de produtividade nem teste de agentes atuais. Dor
relevante: uma pessoa pode precisar de exemplos verificáveis e comparação,
enquanto em outro momento precisa apenas executar um plano conhecido. Evitar
inferir o estado mental automaticamente ou impor uma explicação longa sempre.

### IA7 — Autonomia aceita depende da tarefa e da responsabilidade

[Choudhuri e colegas, julho de 2026, artigo dos autores](https://arxiv.org/abs/2607.00533);
[página institucional](https://www.microsoft.com/en-us/research/publication/you-shall-not-pass-where-and-why-developers-draw-the-line-on-ai-autonomy/).

Estudo misto com 448 profissionais da Microsoft sobre autonomia aceita em
tarefas de engenharia. A maioria aceita IA produzindo trabalho sob supervisão,
mas limites variam por pessoa e tarefa. Responsabilidade se associou a menor
aceitação de agir em nome do profissional; trabalho de identidade, interação
humana e design recebeu menor autonomia aceita. É survey associativo em uma
organização, preprint, não observação causal de comportamento de toda a profissão.
Dor relevante: configurações globais de autonomia podem ser inadequadas para
uma migração de dados, uma exploração descartável e uma decisão arquitetural.
Consentimento genérico não equivale a delegação específica de cada consequência.

### IA8 — Confiança envolve manter o objetivo e diferentes formas de trabalhar

[Choudhuri e colegas, What Needs Attention?, versão 3 de 2025](https://arxiv.org/html/2505.17418v3).

Survey com 238 desenvolvedores de GitHub e Microsoft; modelo PLS-SEM e análise
qualitativa de relatos. Qualidade de sistema/saída, valor funcional e manutenção
de objetivos se associaram à confiança; estilos cognitivos também se relacionaram
à intenção de adoção. O método não identifica causalidade, amostra organizacional
não representa toda população e intenção não equivale a uso eficaz. Dor relevante:
fricção, apresentação e desvio do objetivo importam além da correção de um snippet.
Preferências explícitas por formato e grau de detalhe podem ser úteis, sem
perfil psicológico automático ou diagnóstico do desenvolvedor.

## Oportunidades novas sugeridas por essas dores

Os estudos sustentam a existência de problemas; não testaram estes produtos.
As ideias evitam renomear Radar de premissas, Retomada, Ensaio de mudança,
Laboratório de contexto e Recibo de contexto da pesquisa anterior.

### A — Mapa de obrigações e evidências da alteração

**Cena:** um agente diz que terminou uma migração. O desenvolvedor precisa
verificar rollback, manutenção dos dados e compatibilidade, não apenas testes verdes.
**Produto mínimo:** mapa entre critérios humanos de aceitação, trechos do diff e
evidência ligada à revisão Git: teste, inspeção, referência ou condição não verificada.
Uma sugestão de IA fica marcada como sugestão; prova inexistente permanece ausente.
**Base:** IA1, IA4. **Extensão:** análise de código e persistência de verificação
excedem ADR-0009; requerem caso de uso próprio. **Teste:** revisar dez mudanças
com critérios conhecidos e comparar omissões relevantes e tempo de inspeção.
**Risco:** checklist completo pode criar confiança falsa; não transformar em
selo verde que promete segurança. Não é recibo de entrega de contexto.

### B — Leitura guiada do que mudou

**Cena:** o código funciona, mas quem o aprovou não sabe explicar a condição
de erro, a compensação ou a invariável alterada.
**Produto mínimo:** opção sob demanda que apresenta uma sequência curta de
comportamentos antes/depois com links ao código e pergunta de compreensão
formulada a partir de uma obrigação explícita. Permite testar uma previsão de
saída antes de mostrar a explicação. Sem quiz obrigatório ou nota de funcionário.
**Base:** IA5, IA6. **Teste:** tarefas com biblioteca desconhecida; medir
compreensão da mudança e capacidade de corrigir defeito posterior, além de tempo.
**Risco:** explicação errada; fonte e cobertura precisam ficar visíveis.

### C — Envelope de delegação por tarefa

**Cena:** pode delegar exploração num protótipo, mas quer decidir pessoalmente
troca de arquitetura e alteração de dados.
**Produto mínimo:** declarar objetivo, superfície que o agente pode tocar,
decisões reservadas à pessoa e condições para parar; exportar um briefing e
recuperá-lo na consulta MCP. O Xemnas só consegue orientar e explicar esses
limites; execução coercitiva depende da integração do executor.
**Base:** IA7, IA8. **Teste:** comparar dez tarefas entre instrução global e
envelope específico, contando desvios concretos e atrito de preparação.
**Risco:** sugerir autorização técnica que o executor não aplica. Não é nova
política global automática nem controle de acesso já implementado.

### D — Bancada de exemplos verificados da API

**Cena:** agente e pessoa repetem um exemplo plausível de uma API que tem outra
semântica ou versão instalada.
**Produto mínimo:** um exemplo mínimo humano anexado a referência/versionamento
da dependência e resultado de execução. Recuperação por biblioteca e conceito;
entrada expira quando versão ou comportamento deixa de corresponder.
**Base:** IA4, IA6. **Teste:** tarefas de adoção de cinco APIs desconhecidas,
comparando tentativas falhas e consulta a documentação. **Risco:** executar código
recebido; primeira experiência deve anexar resultados de execução manual, sem
instalar ou rodar automaticamente. Difere de avaliação geral de contexto: a
unidade é uma afirmação específica reproduzível sobre comportamento de API.

### E — Conhecimento adquirido a partir de erro entendido

**Cena:** depois de três tentativas o desenvolvedor descobriu por que cancelamento
não desfaz a gravação. O aprendizado desaparece no histórico de conversa.
**Produto mínimo:** salvar voluntariamente um cartão “interpretação inicial,
contraexemplo, explicação corrigida, exemplo” e vinculá-lo ao conceito/arquivo.
Só aparece quando o conceito volta a ser relevante, sem feed genérico diário.
**Base:** IA5, IA6. **Teste:** verificar uso e entendimento em tarefa posterior,
incluindo cartões que viraram ruído. **Risco:** cristalizar diagnóstico incorreto;
cartão revogável com evidência, sem virar regra de arquitetura automaticamente.
Difere de alternativa arquitetural rejeitada: registra uma compreensão corrigida.

### F — Revisão por unidade de intenção

**Cena:** um diff de vinte arquivos mistura comportamento novo, renomeação e
adaptação de testes; revisar na ordem alfabética esconde o que importa.
**Produto mínimo:** agrupamento sugerido por intenção, relacionando cada unidade
à tarefa e às decisões vigentes. A pessoa corrige grupos; arquivos com múltiplas
intenções e cobertura desconhecida permanecem explícitos.
**Base:** IA1, IA6, IA8. **Teste:** cinco diffs mistos, medir descoberta de
mudanças fora do objetivo e tempo até entendimento. **Risco:** agrupamento pode
ocultar interação entre unidades; sempre manter diff original navegável.
Não é simulação de impacto; organiza uma alteração real para revisão humana.

### G — Contrato de comportamento antes da geração

**Cena:** prompt pede “faça cache” e o agente escolhe silenciosamente expiração,
consistência e comportamento de falha.
**Produto mínimo:** antes de delegar, registrar entradas, saídas, exemplos de
borda e efeitos proibidos definidos pela pessoa, com sugestões baseadas em
contexto confirmado. Exportar esses exemplos como critérios de aceitação,
separados de testes produzidos pelo próprio gerador.
**Base:** IA4, IA6, IA8. **Teste:** dez tarefas vagas com comportamentos
observáveis, medir decisões implícitas indevidas e retrabalho, sem contar número
de perguntas como sucesso. **Risco:** excesso de preparação; deve ser opcional
e curto. Difere de perguntas sobre lacunas na memória: especifica a tarefa
nova antes do código e fornece casos concretos de aceitação.

## Adequação ao Xemnas e limites

Há base de decisões humanas, versões, contexto por tarefa/arquivo e evidências.
Mas analisar diffs, executar snippets, rastrear compreensão ou persistir relatórios
de verificação são extensões, não capacidades comprovadas da revisão atual.
O [ADR-0009](../arquitetura/adr/0009-revisao-consultiva-de-conhecimento.md) prevê
consulta efêmera e não detecção automática de violações no código. Capturas de
sessão não dão autorização para guardar prompts sensíveis; exemplos devem ser
voluntários e redigidos. A captura não confirma a interpretação nem uma regra.

Para começar, C e E têm versões manuais que aproveitam o produto. A e F têm
benefício potencial forte, mas exigem integração com alterações reais. B precisa
de teste de compreensão; tempo sozinho pode recompensar dependência. D requer
controle explícito de execução. Nenhuma dessas prioridades é evidência de compra.
