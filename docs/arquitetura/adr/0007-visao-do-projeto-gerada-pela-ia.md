# ADR-0007: Visão do projeto gerada pela IA

- **Status:** Vigente. (registro original: aceito)
- **Data:** 2026-09-30
- **Contexto:** decisões, regras e mapa respondem "o que vale aqui", mas ninguém lê o projeto inteiro por eles. Faltava uma página que resumisse o projeto e mostrasse os principais fluxos, abrindo cada passo de perto. Montar fluxos à mão seria mais um cadastro; derivá-los só do grafo daria listas, não narrativa.

## Decisão

1. **Resumo e fluxos são escritos pelo provedor de IA configurado** (`application::overview::ProjectOverviews`), a partir de decisões em vigor (com motivo), regras válidas e o mapa (componentes, partes e o que cada um liga). Nada do código nem de capturas cruas é enviado.
2. **Toda frase precisa de fonte.** Cada parágrafo e cada passo cita `D:`/`R:` conhecidos; o que vier sem citação é descartado, e um fluxo precisa de pelo menos dois passos citados. O componente do passo é resolvido pelo nome no mapa; nome desconhecido fica sem vínculo.
3. **Gerada sob pedido e guardada** (migration 16, `project_overviews`). Abrir a aba só lê a versão salva; "Atualizar visão" chama o provedor. A página informa quantas decisões foram confirmadas depois da geração.
4. **Idioma.** A Visão sai no idioma da interface: o app passa a tag (`OutputLanguage`) a `OverviewApi::generate` e o backend a põe na primeira linha da mensagem; nomes de componentes, arquivos, código e ids ficam como escritos ([ADR-0017](0017-polaridade-como-alarme-componentes-na-extracao-e-idioma-da-saida.md)).
5. **Somente leitura.** A visão não é autoridade nem vira contexto do agente; citações abrem em Decisões e componentes no Mapa.

## Consequências

- Os provedores ganham `complete(system, user, schema)` (porta `StructuredModel`), reaproveitada pela extração.
- Sem provedor ativo, a aba mostra o estado vazio com o que será enviado e o erro "Ative um provedor…".
- Remover o projeto apaga a visão.

## Alternativas rejeitadas

- **Fluxos cadastrados à mão:** custo de manutenção alto e desatualização silenciosa.
- **Regenerar ao abrir a aba:** custo e latência a cada visita; o aviso de defasagem basta.
- **Aceitar texto sem citação:** abriria espaço para invenção sem como conferir.
