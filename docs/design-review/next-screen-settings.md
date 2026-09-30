# Próxima tela: Configurações

Plano registrado em 30/09/2026. Regra da etapa: só interface; o backend não muda.
Cada seção entra apenas com o que o contrato atual oferece.

## Entrada e estrutura

- Configurações é do app, não do projeto: abre por um ícone de engrenagem na
  barra de título, ao lado do tema, e ocupa a área abaixo dela (lateral de
  projetos e barra do projeto saem de cena). "Voltar aos projetos" e o mesmo
  ícone fecham a página e devolvem a seleção anterior.
- Página com navegação à esquerda (seções) e coluna de leitura de até 720 px.
- A janela usa o material do sistema (Mica Alt/acrílico); nada na página depende
  de blur.

## Seções, em ordem

| Seção | Backend | O que entra agora |
| --- | --- | --- |
| **IA e privacidade** | Pronto no `master`: `application::profile::AiSettings` (`load_or_seed`, `save`, `set_secret`, `secret`, `grant`, `revoke`), `build_preview`, `consent_status`, `choose_extractor`; cofre via `ai_provider::KeyringSecretStore` | Tudo, descrito abaixo |
| **Diagnóstico** | Parcial no `master`: `Diagnostics::export`, `Jobs::list`, `Jobs::reprocess`. Teto de tentativas e retenção de rejeitados da outbox só no branch de backend | Próxima etapa, sem "perdas" até o merge |
| **OpenCode** | Status e teste de conexão só no branch de backend | Não entra até o merge |

## IA e privacidade

1. **Estado atual**, no topo, com selo e uma frase: *Local, sem rede* (extrator
   offline), *Provedor externo ativo* (consentimento válido) ou *Provedor externo
   bloqueado* com o motivo de `consent_status`. Bloqueado significa que as
   capturas não são analisadas até resolver; não há envio sem consentimento.
2. **Extrator**: escolha entre *Local* e *Compatível com OpenAI*. Para o externo:
   endereço (HTTPS, ou HTTP só em IP de loopback), modelo e limite de caracteres
   por item. Validação é a de `AiProfile::validate`, mostrada junto do formulário.
   **Salvar configuração** só com alteração válida. Salvar uma alteração apaga o
   consentimento anterior (a prévia mudou), e a tela diz isso antes de salvar.
3. **Chave do provedor**: campo mascarado e **Guardar no cofre**. A chave salva
   nunca é lida para a tela; só se mostra se existe. O campo se esvazia após
   guardar.
4. **O que sai da máquina**: a prévia de `build_preview` — destino (só o host),
   modelo, categorias com limite por item, total aproximado por análise e a
   redação de segredos feita na captura.
5. **Consentimento**: requisitos visíveis (configuração salva, chave no cofre);
   **Consentir e ativar** grava o consentimento ligado ao hash da prévia. Com
   consentimento ativo: data e **Revogar**, com confirmação explícita, porque
   revogar desliga as chamadas e apaga a chave do cofre.

Estados: carregando (sem valores provisórios), falha de leitura com Tentar de
novo, operação em andamento bloqueia repetição, erros de validação em linguagem
de produto; falha de E/S ou do cofre vira mensagem genérica e o detalhe vai só
para o log. Todo acesso a arquivo e cofre roda fora da thread de UI.

Fora desta etapa: teste de conexão com o provedor (não existe caso de uso),
múltiplos perfis, escolha de catálogo de modelos.

## Estado da implementação (30/09/2026)

IA e privacidade implementada em `apps/desktop-gpui/src/screens/settings.rs`,
com o campo mascarado em `ui/search_field.rs` (`SearchField::secret`).
Validado no modo `--demo` (perfil e cofre em memória) renderizado em Linux/Xvfb:
salvar, bloqueio com motivo, guardar chave, consentir, revogar com confirmação.
Imagens: [local](settings-ai-local.png), [externo aguardando consentimento](settings-ai-external.png).
Os controles de janela aparecem quebrados nessas imagens porque usam a fonte
Segoe Fluent Icons, que só existe no Windows. Não houve verificação nativa no
Windows nem do cofre de credenciais real nesta etapa.

Para rodar no Linux (só para inspeção): `cargo run -p desktop-gpui --bin xemnas
--features rfd/xdg-portal,gpui_platform/x11 -- --demo`, com `libxkbcommon-x11-dev`
e um driver Vulkan (por exemplo `mesa-vulkan-drivers`).
