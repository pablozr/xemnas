# Dores de desenvolvimento: contratos, linguagem e reprodução

**Data:** 02/10/2026.
**Pergunta:** que evidências empíricas sobre linguagem, dependências, requisitos
e reprodução de falhas podem orientar extensões da memória decisional?
**Status:** Aberta. Fontes consultadas; propostas e experimentos ainda não executados.

## Método

Pesquisa dirigida de estudos originais, páginas dos autores/instituições e versões
abertas de artigos. Não é revisão sistemática: não há corpus exaustivo, protocolo
de seleção pré-registrado ou metanálise. A unidade de análise, o método e o limite
de cada fonte estão abaixo. Fonte empírica fundamenta a dor naquele contexto;
não demonstra demanda, eficácia ou exclusividade de um recurso do Xemnas.

As propostas são inferências nossas. Não alteram o [MVP](../produto/MVP-SPEC.md)
nem o [vocabulário](../produto/CONTEXT.md). O recorte favorece conhecimento e
evidência ligados à tarefa; não substitui depurador, análise estática ou CI.

## C01 — Escolher palavras não garante entendimento compartilhado

Feitelson et al., **How Developers Choose Names**, aceito em IEEE TSE;
[preprint dos autores, 2021](https://arxiv.org/abs/2103.07487) e
[texto integral](https://arxiv.org/pdf/2103.07487).

**Método/amostra:** sequência de experimentos com 334 participantes no total,
estudantes e profissionais; cenários de programação e avaliação de nomes.
**Achado:** nomes escolhidos variam muito; os autores distinguem conceito,
palavra e composição. Entender um nome fornecido e escolher espontaneamente o
mesmo nome são tarefas diferentes. **Limite:** muitos participantes são estudantes;
cenários curtos e contexto linguístico específico não representam toda manutenção.
**Inferência:** glossário projeto → conceito → símbolos, com definições confirmadas
e exemplos, poderia reduzir desencontro entre agentes e humanos. Não impor rename
nem decidir que termos parecidos significam a mesma coisa.

## C02 — Migrar API exige conhecer o contrato usado

Xavier, Brito, Hora e Valente, **Historical and Impact Analysis of API Breaking
Changes**, SANER 2017;
[artigo disponibilizado pela UFMG](https://homepages.dcc.ufmg.br/~mtov/pub/2017-saner-breaking-apis.pdf).

**Método/amostra:** mineração de 317 bibliotecas Java, cerca de 9 mil releases e
260 mil aplicações clientes. **Achado:** há mudanças incompatíveis e impacto
potencial nos consumidores; o impacto não pode ser inferido apenas da existência
de uma nova versão. **Limite:** análise sobretudo de compatibilidade sintática;
imports de tipos são aproximação do uso e podem superestimar impacto. A
classificação trata remoção de elemento já deprecated de modo próprio; não copiar
essa convenção como regra universal de compatibilidade. **Inferência:** um dossiê
de migração ligando versões, comportamento usado, decisão e testes específicos
ajudaria a conferir o contrato efetivamente necessário. Não prometer descoberta
completa de breaking changes sem ferramenta própria e cobertura declarada.

## C03 — A dificuldade de depurar envolve hipótese e ambiente

Layman et al., **Debugging Revisited**, ESEM 2013;
[página dos autores na Microsoft Research](https://www.microsoft.com/en-us/research/publication/debugging-revisited-toward-understanding-debugging-needs-contemporary-software-developers/).

**Método/amostra:** entrevistas codificadas com 15 engenheiros da Microsoft.
**Achado:** relação entre hipótese, instrumentação e ambiente, informação dos logs
e execução concorrente aparecem como desafios. **Limite:** qualitativo, uma
empresa, ferramentas e práticas de 2013; identifica mecanismos, não prevalência
atual. A página institucional aberta fornece o resumo; não se atribuem efeitos
numéricos não conferidos. **Inferência:** preservar a configuração e a observação
que sustentam uma hipótese é diferente de guardar apenas a solução final.

## C04 — Informação insuficiente prolonga investigação não reproduzível

Rahman, Khomh e Castelluccio, **Why are Some Bugs Non-Reproducible?**, 2021;
[preprint dos autores](https://arxiv.org/abs/2108.05316).

**Método/amostra:** 576 relatos não reproduzíveis em Firefox/Eclipse e estudo
com 13 desenvolvedores profissionais. **Achado:** fatores de não reprodução e
busca adicional de informação; fechar ou solicitar esclarecimento são estratégias
observadas. **Limite:** dois sistemas, seleção de casos não reproduzíveis e estudo
pequeno. Não estima a frequência de falhas em todos os projetos. Abertura do
abstract verificada; não atribuímos detalhe metodológico do texto completo.
**Inferência:** cápsula com passos, ambiente, esperado/observado e lacunas ajuda
a transferir uma investigação sem tornar hipótese em causa confirmada.

## C05 — Reprodução tem três dimensões complementares

Johnson et al., **An Empirical Investigation into the Reproduction of Bug Reports
for Android Apps**, SANER 2022;
[versão dos autores](https://arxiv.org/abs/2301.01235).

**Método/amostra:** análise de 180 relatos reproduzíveis de apps Android no GitHub.
**Achado:** ambiente, passos para reproduzir e comportamento observado são eixos
examinados ao reconstruir a falha. **Limite:** casos selecionados como reproduzíveis,
ecossistema Android; não fornecem taxa geral de sucesso nem comparação do Xemnas.
Resumo verificado; link do PDF hospedado na universidade falhou nesta consulta.
**Inferência:** conferir esses eixos antes de pedir nova implementação pode tornar
um handoff mais útil, sem exigir gravação contínua de tela ou conteúdo sensível.

## C06 — Um alerta tecnicamente plausível pode ser impraticável

Johnson et al., **Why Don't Software Developers Use Static Analysis Tools to Find
Bugs?**, ICSE 2013;
[registro dos autores no Google Research](https://research.google/pubs/why-dont-software-developers-use-static-analysis-tools-to-find-bugs/).

**Método/amostra:** entrevistas com 20 desenvolvedores. **Achado:** falsos positivos
e apresentação dos avisos, entre outras barreiras, dificultam adoção apesar do
benefício percebido. **Limite:** entrevista não mede economia realizada; ferramentas
de 2013 não equivalem a agentes atuais. Resumo institucional verificado.
**Inferência:** Xemnas deve recuperar o motivo de um aviso aceito/dispensado,
com escopo, evidência e correção posterior. Não silenciar verificações nem aprovar
código porque o aviso já foi dispensado em outro contexto.

## C07 — Requisito ambíguo é sinal a investigar, não defeito certificado

Femmer et al., **Rapid Quality Assurance with Requirements Smells**, JSS 2017;
[manuscrito dos autores](https://wwwbroy.in.tum.de/~femmer/works/2016-requirements_smells-jss.pdf).

**Método/amostra:** estudo exploratório de múltiplos casos: três contextos
industriais e um acadêmico, com inspeções e avaliação de achados. **Achado:**
smells de linguagem podem indicar problemas relevantes, mas a precisão e a
relevância variam; contexto e posição do trecho importam. **Limite:** indicadores
não são prova de erro e não equivalem a compreensão semântica completa. Nem toda
pergunta de pesquisa foi avaliada em todo caso. **Inferência:** exemplos concretos
de aceitação/rejeição ajudam a explicitar “rápido”, “seguro” ou “compatível” antes
de delegar; o usuário mantém a definição do objetivo.

## C08 — Qualidade e produtividade exigem evidência contextual

Cheng et al., **What Improves Developer Productivity at Google? Code Quality.**,
FSE Industry 2022;
[registro](https://research.google/pubs/what-improves-developer-productivity-at-google-code-quality/) e
[artigo integral](https://storage.googleapis.com/gweb-research2023-media/pubtools/6862.pdf).

**Método:** análise de dados em painel e painel defasado de desenvolvedores Google,
considerando 39 fatores. **Achado:** os autores encontram evidência longitudinal
ligando qualidade percebida e produtividade percebida; examinam dívida, ferramentas
e fatores organizacionais. **Limite:** medição de produtividade individual por
survey, fatores disponíveis incompletos, uma empresa; desenho não é experimento
randomizado de uma ferramenta. Não derivar retorno financeiro do Xemnas nem
explicar qualidade por número de commits. **Inferência:** registrar episódios de
fricção e evidência da dívida torna a discussão mais concreta que um score genérico.

## Síntese para produto

Estas fontes sustentam testar glossário semântico, dossiê de migração, cápsula de
reprodução, memória de triagem de avisos e critérios exemplificados. O estudo de
hipóteses de debugging está no [catálogo de cognição](dores-dev-cognicao-fontes.md);
coordenação e dívida estão no [catálogo de equipes](dores-dev-conhecimento-equipe-fontes.md).
A síntese das vinte propostas terá experimentos próprios, não resultados destes
papers apresentados como eficácia do produto.

## Limitações de acesso e método

Não usados como evidência: resumos de busca de documentos que não abriram; página
ORBilu de Paska e PDF universitário de reprodução Android falharam. O último foi
substituído pelo abstract original aberto no arXiv. Não há entrevistas próprias,
benchmark do Xemnas ou licença concedida para reproduzir os artigos; apenas links
e sínteses curtas. Nenhuma coleta adicional, integração ou código implementado.
