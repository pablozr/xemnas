# ADR-0010 — Qualificadores persistentes e fronteira de envio

Status: implementado no lote de confiança/clareza, com limitações de validação
registradas em [operação](../../operacao/lote-confianca-clareza.md).

Ressalvas de autoria, alcance e validação não podem depender de sobreviver ao resumo
do motivo. Persistimos qualificadores separados nos candidatos, decisões/revisões,
claims e sugestões. Extração exige suporte literal; edição preserva citações
inalteradas e registra texto novo como declaração humana. Ausência é desconhecida,
não autoridade global. Isso acrescenta migrations/contratos, em vez de tentar
resolver perda de alcance somente por prompt ou apresentação.

Claims derivadas conservam versão e scope da origem; confirmação compara o snapshot
original transacionalmente. Contexto entrega escolha/restrições como unidade inteira
ou declara omissão. JSON malformado é erro, não lista vazia. Legado não é enriquecido
automaticamente. Este contrato não corrige reconstrução histórica `as_of`.

Conteúdo externo passa por proteção de padrões conhecidos antes dos cortes,
decodificando strings JSON. Perfil, consentimento e credencial são revalidados antes
de chamadas/retries, inclusive depois do refresh ChatGPT; rotação reconhecida mantém
linhagem da sessão sem autorizar substituição arbitrária. Não há payload integral
em auditoria, certificação de detecção de todo segredo ou promessa de abortar HTTP
já iniciado. Prévia ampliada requer reconsentimento.

Assessment registra motivo, classificações e tentativa atomicamente; projeção
por Project não apresenta resultado de tentativa antiga durante retry. Interface
consome esse caso de uso, sem classificar capturas por contagens globais.

A confirmação normativa vigente continua humana. Memória descritiva automática,
revisão por exceção e context router são direção futura, não efeitos deste ADR.
