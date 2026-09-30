# Próxima tela: Configurações

Plano de 30/09/2026. Proposta de produto e layout ancorada nos contratos que já existem; nada aqui foi implementado.

## Recomendação

Implementar **Configurações** como um lugar do app (não do projeto), com três seções, entregues nesta ordem:

1. **IA e privacidade**: backend completo no ticket 13. É o que muda o que o produto faz: hoje todo candidato vem do extrator offline, e a única forma de ligar um provedor real é editar `ai-profile.json` à mão.
2. **Diagnóstico**: backend completo no ticket 11. A linha de estado da lateral já mostra "2 falhas" e "Captura indisponível"; falta o lugar onde a pessoa entende e age (reprocessar, exportar).
3. **Integração OpenCode**: backend parcial. Entra depois, só com o que for real.

A especificação lista as três como telas obrigatórias do MVP ([MVP-SPEC §8](../MVP-SPEC.md)).

## Onde fica na navegação

- Ícone de engrenagem na barra de título, ao lado do tema, com tooltip "Configurações" e atalho `Ctrl ,`.
- Na paleta (Ctrl K): "Configurações", "IA e privacidade", "Diagnóstico".
- A linha de estado da lateral vira um link: clicar em "2 falhas" abre Diagnóstico; clicar em "Captura indisponível" abre Integração.
- Ao abrir, a lateral de projetos continua visível (orientação espacial) e a barra de projeto dá lugar a uma trilha "Configurações / IA e privacidade". Esc ou a trilha voltam ao projeto.

## Layout

```
┌ título ────────────────────────────────────────────────────────────────┐
├ projetos (248) ┬ Configurações / IA e privacidade ─────────────────────┤
│                │ ┌ seções (200) ┐ ┌ coluna de leitura (760) ─────────┐ │
│                │ │ IA e priv.   │ │ Extrator em uso                  │ │
│                │ │ Diagnóstico  │ │ Provedor · modelo · limites      │ │
│                │ │ OpenCode     │ │ O que sai da máquina (prévia)    │ │
│                │ └──────────────┘ │ Consentimento                    │ │
│                │                  └──────────────────────────────────┘ │
│ ● Captura ativa│                    [ rodapé de ações fixo ]           │
└────────────────┴───────────────────────────────────────────────────────┘
```

Reaproveita o que já existe: `reading_page`, `form_field`, `section_label`, `action_footer`, `status_pill`, `error_banner`, `toast`, `empty_panel`, `skeleton_list`, `action_button` e a lista de seções com `mark_selected` e hover com mola. Nenhum componente novo além de um **seletor segmentado** (Offline / Provedor compatível) e um **campo de segredo** (mascarado, nunca reexibido).

## Seção 1: IA e privacidade

| Necessidade | Contrato hoje | Observação |
| --- | --- | --- |
| Estado atual | `AiSettings::status()` → `ExtractorChoice` (OfflineFake, ExternalEnabled, ExternalBlocked) e `has_secret` | Cabeçalho da seção com selo: "Offline", "Provedor ativo" ou "Bloqueado: consentimento ausente" (texto de `consent_status`) |
| Carregar e salvar perfil | `load_or_seed()`, `save(&AiProfile)`, `AiProfile::validate()` | Erros de validação já vêm em português ("o modelo é obrigatório"); mostrar no rodapé |
| Tipo, modelo, endpoint e limite | `AiProfile { kind, model, endpoint, max_input_chars }` | Endpoint aceita HTTPS; HTTP só em loopback. Explicar isso na dica do campo |
| Chave do provedor | `set_secret(id, secret)`, `secret(id)` no cofre do sistema | Campo de senha que só grava. Mostrar "Chave guardada no cofre do Windows", nunca o valor |
| O que sai da máquina | `preview(&profile)` → `ConsentPreview` (host, modelo, categorias com limite por item, redação na ingestão, total aproximado) | Tabela curta por categoria (Mensagem do usuário, Resposta do assistente, Trecho de alteração, Resultado de ferramenta). É o coração da seção |
| Consentir | `grant(&profile, &preview, now)` exige segredo salvo | Botão primário "Permitir chamadas para {host}" só aparece com prévia válida e chave salva |
| Revogar | `revoke(&profile)` desliga, apaga a chave e salva | Ação secundária destrutiva com confirmação em linha, como remover projeto |
| Consentimento invalidado | `consent_status` detecta mudança de endpoint, modelo, limite ou tipo | Ao editar um campo depois do consentimento, avisar antes de salvar: "Salvar exige consentir de novo" |

**Estados:** offline (padrão, sem nada a fazer: dizer o que o extrator offline cobre), editando (rodapé com Cancelar / Salvar), prévia pronta sem consentimento, provedor ativo, bloqueado, erro do cofre.

**Lacuna real:** a especificação pede "teste com resposta estruturada". Não há caso de uso de teste de conexão no `application`; o `ai-provider` só expõe o cliente de extração. Propor `AiSettings::probe(profile)` que faz uma chamada mínima e devolve um resultado sanitizado, com teste. Até lá, a tela não mostra botão de teste.

## Seção 2: Diagnóstico

| Necessidade | Contrato hoje | Observação |
| --- | --- | --- |
| Visão geral | `Diagnostics::export()` → `DiagnosticsDocument` | Uma leitura por abertura, em segundo plano, com esqueleto enquanto carrega |
| Contagens | `counts` (projetos, capturas, artefatos, candidatos por estado, decisões, revisões, jobs por estado) | Linha de números com rótulo, sem gráficos |
| Perdas e ruído | `metrics.losses`, `metrics.noise` | Frases: "3 análises falharam · 1 captura rejeitada pela outbox"; proporção de rejeitados como número, não gauge |
| Latência | `metrics.latency_capture_to_candidate_ms`, `review_time_ms` (`Distribution`) | Mostrar mediana e p90 se existirem; esconder quando `samples == 0` |
| Jobs recentes | `recent_jobs` (id, tipo, estado, tentativas, atualizado, `error_code`) | Lista com `status_pill`; falhas no topo |
| Reprocessar | `Jobs::reprocess(id)` (failed → queued, limpa o diagnóstico) | Ação por linha de job falho; toast "Job reenfileirado" |
| Recibos de captura | `recent_receipts` (sem conteúdo) | Lista discreta: captura, artefatos, recebido, projeto |
| Perfil de IA | `ai_profile` (tipo, provedor, segredo presente, consentimento) | Link para a seção 1 |
| Exportar sanitizado | `DiagnosticsDocument` é serializável | Ação "Exportar diagnóstico…" com seletor de arquivo nativo, igual à exportação de decisões |
| Abrir pastas | caminhos conhecidos no `main.rs` (dados, outbox, logs) | Ações "Abrir pasta de logs/outbox" via shell do sistema; a UI recebe os caminhos prontos da composição |

**Pré-requisito de composição:** o `main.rs` já cria `AiSettings`; falta passar ao `Shell` um `Diagnostics` e um `Jobs` (só `reprocess`/`list`) pelos mesmos ports, sem a UI conhecer SQLite.

## Seção 3: Integração OpenCode

Disponível: `DiscoveryInfo` (versão de protocolo, porta, instância) escrito pelo `local-api`, checkpoints por sessão (`CaptureCheckpointRecord`: adapter, sessão, última observação), contagens da outbox. Falta: versão do OpenCode detectada e compatibilidade, e um teste de conexão iniciado pela UI. Proposta: mostrar só "API local na porta N, protocolo v1", "Última captura há X" e "Outbox: N pendentes", e deixar versão e teste para um ticket de backend.

## Ordem de entrega

1. Destino Configurações no shell (engrenagem, `Ctrl ,`, paleta, trilha, Esc) com as três seções na navegação; só IA e privacidade habilitada.
2. IA e privacidade completa, sem teste de conexão.
3. Diagnóstico somente leitura, depois reprocessar e exportar.
4. Linha de estado da lateral clicável para Diagnóstico e Integração.
5. Ticket de backend: `AiSettings::probe` e detecção de versão do OpenCode. Depois disso, o botão de teste e a seção OpenCode completa.

## Gate visual

Capturar o binário novo com: perfil offline, edição com erro de validação, prévia sem consentimento, provedor ativo, consentimento invalidado por edição, Diagnóstico com e sem falhas, e reprocessar. Nas duas paletas, janela padrão e compacta. Nenhuma tela mostra a chave, o conteúdo de captura ou o texto técnico de erro.
