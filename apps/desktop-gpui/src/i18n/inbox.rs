//! Copy for the inbox area. See [`crate::i18n`] for how entries are declared.

use super::{current, formats, french_one, one, russian_form, strings, Language};

/// Declares a counted noun phrase: one `fn name(count) -> String` whose
/// literals name `count` inline. English, Portuguese, Spanish, French, German
/// and Italian give a singular and a plural, Russian its three forms, and
/// Japanese, Chinese and Korean one literal.
macro_rules! plurals {
    ($(
        $(#[$meta:meta])*
        $name:ident($count:ident) {
            en: [$en1:literal, $en2:literal], pt: [$pt1:literal, $pt2:literal],
            es: [$es1:literal, $es2:literal], fr: [$fr1:literal, $fr2:literal],
            de: [$de1:literal, $de2:literal], it: [$it1:literal, $it2:literal],
            ja: $ja:literal, zh: $zh:literal, ko: $ko:literal,
            ru: [$ru1:literal, $ru2:literal, $ru3:literal] $(,)?
        }
    )*) => {$(
        $(#[$meta])*
        pub fn $name($count: usize) -> String {
            match current() {
                Language::English => {
                    if one($count) { format!($en1) } else { format!($en2) }
                }
                Language::Portuguese => {
                    if one($count) { format!($pt1) } else { format!($pt2) }
                }
                Language::Spanish => {
                    if one($count) { format!($es1) } else { format!($es2) }
                }
                Language::French => {
                    if french_one($count) { format!($fr1) } else { format!($fr2) }
                }
                Language::German => {
                    if one($count) { format!($de1) } else { format!($de2) }
                }
                Language::Italian => {
                    if one($count) { format!($it1) } else { format!($it2) }
                }
                Language::Japanese => format!($ja),
                Language::Chinese => format!($zh),
                Language::Korean => format!($ko),
                Language::Russian => match russian_form($count) {
                    0 => format!($ru1),
                    1 => format!($ru2),
                    _ => format!($ru3),
                },
            }
        }
    )*};
}

strings! {
    /// Source kind: a change hunk of a diff.
    source_diff_hunk { en: "Change hunk", pt: "Trecho de alteração",
        es: "Fragmento de cambio", fr: "Extrait de modification",
        de: "Änderungsabschnitt", it: "Blocco di modifica", ja: "変更の差分ブロック",
        zh: "变更片段", ko: "변경 조각", ru: "Фрагмент изменения" }
    /// Source kind: a message from the user.
    source_user_text { en: "User message", pt: "Mensagem do usuário",
        es: "Mensaje del usuario", fr: "Message de l’utilisateur",
        de: "Nachricht des Nutzers", it: "Messaggio dell’utente",
        ja: "ユーザーのメッセージ", zh: "用户消息", ko: "사용자 메시지",
        ru: "Сообщение пользователя" }
    /// Source kind: an assistant reply.
    source_assistant_text { en: "Assistant reply", pt: "Resposta do assistente",
        es: "Respuesta del asistente", fr: "Réponse de l’assistant",
        de: "Antwort des Assistenten", it: "Risposta dell’assistente",
        ja: "アシスタントの返信", zh: "助手回复", ko: "어시스턴트 응답",
        ru: "Ответ ассистента" }
    /// Source kind: the result of a tool.
    source_tool_summary { en: "Tool result", pt: "Resultado de ferramenta",
        es: "Resultado de la herramienta", fr: "Résultat d’outil",
        de: "Werkzeugergebnis", it: "Risultato dello strumento", ja: "ツールの結果",
        zh: "工具结果", ko: "도구 결과", ru: "Результат инструмента" }
    /// Source kind: an export document.
    source_export_document { en: "Export document", pt: "Documento de exportação",
        es: "Documento de exportación", fr: "Document d’export",
        de: "Exportdokument", it: "Documento di esportazione",
        ja: "エクスポートドキュメント", zh: "导出文档", ko: "내보내기 문서",
        ru: "Документ экспорта" }
    /// Source kind: a project document.
    source_document { en: "Project document", pt: "Documento do projeto",
        es: "Documento del proyecto", fr: "Document du projet", de: "Projektdokument",
        it: "Documento del progetto", ja: "プロジェクトのドキュメント",
        zh: "项目文档", ko: "프로젝트 문서", ru: "Документ проекта" }
    /// Source kind: anything else a capture brought.
    source_capture { en: "Capture source", pt: "Fonte da captura",
        es: "Fuente de la captura", fr: "Source de la capture",
        de: "Erfassungsquelle", it: "Fonte dell’acquisizione",
        ja: "キャプチャのソース", zh: "捕获来源", ko: "캡처 출처", ru: "Источник захвата" }
    /// What an export source holds when it has no line numbers.
    source_export_exact { en: "exact content to save", pt: "conteúdo exato para salvar",
        es: "contenido exacto para guardar", fr: "contenu exact à enregistrer",
        de: "genauer Inhalt zum Speichern", it: "contenuto esatto da salvare",
        ja: "保存する内容そのまま", zh: "将要保存的完整内容", ko: "저장할 내용 그대로",
        ru: "точное содержимое для сохранения" }
    /// What a prose source is when it has no line numbers.
    source_capture_text { en: "capture text", pt: "texto da captura",
        es: "texto de la captura", fr: "texte de la capture", de: "Text der Erfassung",
        it: "testo dell’acquisizione", ja: "キャプチャのテキスト", zh: "捕获文本",
        ko: "캡처 텍스트", ru: "текст захвата" }

    /// Editor field label: the decision's question.
    editor_question { en: "Question", pt: "Pergunta", es: "Pregunta", fr: "Question",
        de: "Frage", it: "Domanda", ja: "質問", zh: "问题", ko: "질문", ru: "Вопрос" }
    /// Editor field label: the suggested choice.
    editor_choice { en: "Suggested choice", pt: "Escolha sugerida",
        es: "Elección sugerida", fr: "Choix suggéré", de: "Vorgeschlagene Wahl",
        it: "Scelta suggerita", ja: "提案された選択", zh: "建议的选择",
        ko: "제안된 선택", ru: "Предлагаемый выбор" }
    /// Editor field label: the reason.
    editor_reason { en: "Reason", pt: "Motivo", es: "Motivo", fr: "Raison", de: "Grund",
        it: "Motivo", ja: "理由", zh: "原因", ko: "이유", ru: "Причина" }
    /// Hint under the choice field.
    editor_choice_hint { en: "The choice that will be recorded if you confirm.",
        pt: "A escolha que será registrada se você confirmar.",
        es: "La elección que se registrará si confirmas.",
        fr: "Le choix qui sera enregistré si vous confirmez.",
        de: "Die Wahl, die gespeichert wird, wenn du bestätigst.",
        it: "La scelta che verrà registrata se confermi.",
        ja: "確定すると記録される選択です。",
        zh: "确认后将被记录的选择。",
        ko: "확인하면 기록될 선택이에요.",
        ru: "Выбор, который будет записан, если вы подтвердите." }
    /// Hint under the reason field.
    editor_reason_hint { en: "Why this choice was made, in the team's words.",
        pt: "Por que essa escolha foi feita, nas palavras da equipe.",
        es: "Por qué se tomó esta elección, con las palabras del equipo.",
        fr: "Pourquoi ce choix a été fait, dans les mots de l’équipe.",
        de: "Warum diese Wahl getroffen wurde, in den Worten des Teams.",
        it: "Perché è stata fatta questa scelta, con le parole del team.",
        ja: "この選択をした理由を、チームの言葉で。",
        zh: "做出这个选择的原因，用团队自己的话说明。",
        ko: "이 선택을 한 이유를 팀의 말로 적어요.",
        ru: "Почему сделан этот выбор, словами команды." }
    /// Editor title.
    editor_title { en: "Adjust candidate", pt: "Ajustar candidato",
        es: "Ajustar candidato", fr: "Ajuster le candidat", de: "Kandidat anpassen",
        it: "Modifica candidato", ja: "候補を調整", zh: "调整候选", ko: "후보 조정",
        ru: "Скорректировать кандидата" }
    /// Editor intro.
    editor_intro { en: "Review the question, the choice and the reason before confirming.",
        pt: "Revise a pergunta, a escolha e o motivo antes de confirmar.",
        es: "Revisa la pregunta, la elección y el motivo antes de confirmar.",
        fr: "Relisez la question, le choix et la raison avant de confirmer.",
        de: "Prüfe Frage, Wahl und Grund, bevor du bestätigst.",
        it: "Controlla la domanda, la scelta e il motivo prima di confermare.",
        ja: "確定する前に、質問・選択・理由を確認してください。",
        zh: "确认前请检查问题、选择和原因。",
        ko: "확인하기 전에 질문, 선택, 이유를 살펴보세요.",
        ru: "Проверьте вопрос, выбор и причину перед подтверждением." }
    /// Editor section with the sources to consult.
    editor_evidence_title { en: "Evidence for reference", pt: "Evidências para consulta",
        es: "Evidencias para consulta", fr: "Preuves à consulter",
        de: "Belege zum Nachschlagen", it: "Evidenze da consultare",
        ja: "参照用の根拠", zh: "参考证据", ko: "참고용 근거",
        ru: "Доказательства для справки" }
    /// Editor action: leave without saving.
    editor_cancel { en: "Cancel", pt: "Cancelar", es: "Cancelar", fr: "Annuler",
        de: "Abbrechen", it: "Annulla", ja: "キャンセル", zh: "取消", ko: "취소",
        ru: "Отмена" }
    /// Editor action: save the edits only.
    editor_save_edits { en: "Save changes", pt: "Salvar ajustes",
        es: "Guardar ajustes", fr: "Enregistrer les ajustements",
        de: "Änderungen speichern", it: "Salva modifiche", ja: "調整を保存",
        zh: "保存调整", ko: "조정 저장", ru: "Сохранить правки" }
    /// Editor action: save the edits and confirm.
    editor_save_confirm { en: "Save and confirm", pt: "Salvar e confirmar",
        es: "Guardar y confirmar", fr: "Enregistrer et confirmer",
        de: "Speichern und bestätigen", it: "Salva e conferma", ja: "保存して確定",
        zh: "保存并确认", ko: "저장하고 확인", ru: "Сохранить и подтвердить" }
    /// Editor footer while saving.
    editor_saving { en: "Saving…", pt: "Salvando…", es: "Guardando…",
        fr: "Enregistrement…", de: "Speichern …", it: "Salvataggio…", ja: "保存中…",
        zh: "正在保存…", ko: "저장 중…", ru: "Сохранение…" }
    /// Qualifier kind: attribution.
    qualifier_attribution { en: "Attribution", pt: "Atribuição", es: "Atribución",
        fr: "Attribution", de: "Zuschreibung", it: "Attribuzione", ja: "帰属",
        zh: "归属", ko: "출처 표기", ru: "Атрибуция" }
    /// Qualifier kind: scope.
    qualifier_scope { en: "Scope", pt: "Escopo", es: "Alcance", fr: "Portée",
        de: "Geltungsbereich", it: "Ambito", ja: "範囲", zh: "范围", ko: "범위",
        ru: "Область" }
    /// Qualifier kind: validation.
    qualifier_validation { en: "Validation", pt: "Validação", es: "Validación",
        fr: "Validation", de: "Validierung", it: "Convalida", ja: "検証", zh: "验证",
        ko: "검증", ru: "Валидация" }
    /// Section title of the scope and caveats.
    qualifiers_title { en: "Scope and caveats", pt: "Alcance e ressalvas",
        es: "Alcance y salvedades", fr: "Portée et réserves",
        de: "Geltungsbereich und Vorbehalte", it: "Ambito e riserve",
        ja: "範囲と留意点", zh: "范围与限制说明", ko: "범위와 유의 사항",
        ru: "Область и оговорки" }
    /// Where a qualifier came from: quoted from the evidence.
    qualifier_cited { en: "cited from the evidence", pt: "citado da evidência",
        es: "citado de la evidencia", fr: "cité à partir de la preuve",
        de: "aus dem Beleg zitiert", it: "citato dall’evidenza", ja: "根拠からの引用",
        zh: "引自证据", ko: "근거에서 인용", ru: "цитата из доказательства" }
    /// Where a qualifier came from: written by the reviewer.
    qualifier_written { en: "written in the review, no source",
        pt: "escrito na revisão, sem fonte",
        es: "escrito en la revisión, sin fuente",
        fr: "écrit lors de la revue, sans source",
        de: "in der Prüfung verfasst, ohne Quelle",
        it: "scritto nella revisione, senza fonte",
        ja: "レビューで記入、ソースなし", zh: "审阅时填写，无来源",
        ko: "검토 중에 작성, 출처 없음", ru: "написано при проверке, без источника" }
    /// Note above the qualifier fields in the editor.
    qualifiers_edit_note { en: "Changing a quote makes it the reviewer's statement, with no verified source. Delete the text to remove it.",
        pt: "Alterar uma citação a torna declaração do revisor, sem fonte verificada. Apague o texto para remover.",
        es: "Cambiar una cita la convierte en una declaración del revisor, sin fuente verificada. Borra el texto para quitarla.",
        fr: "Modifier une citation en fait une déclaration du relecteur, sans source vérifiée. Effacez le texte pour la retirer.",
        de: "Wenn du ein Zitat änderst, wird es zur Aussage der prüfenden Person, ohne verifizierte Quelle. Lösche den Text, um es zu entfernen.",
        it: "Modificare una citazione la trasforma in una dichiarazione del revisore, senza fonte verificata. Cancella il testo per rimuoverla.",
        ja: "引用を変更すると、検証済みのソースがないレビュー担当者の記述になります。削除するにはテキストを消してください。",
        zh: "修改引文后，它将成为审阅者的声明，没有经过验证的来源。清空文本即可删除。",
        ko: "인용을 바꾸면 검증된 출처가 없는 검토자의 진술이 돼요. 지우려면 텍스트를 비워 주세요.",
        ru: "Если изменить цитату, она станет заявлением проверяющего без подтверждённого источника. Чтобы удалить, сотрите текст." }
    /// Hint of a qualifier quoted from the evidence.
    qualifier_quote_hint { en: "Quote from the evidence; check the source before changing it.",
        pt: "Citação da evidência; consulte a fonte antes de alterar.",
        es: "Cita de la evidencia; consulta la fuente antes de cambiarla.",
        fr: "Citation de la preuve ; consultez la source avant de la modifier.",
        de: "Zitat aus dem Beleg; sieh dir die Quelle an, bevor du es änderst.",
        it: "Citazione dall’evidenza; consulta la fonte prima di modificarla.",
        ja: "根拠からの引用です。変更する前にソースを確認してください。",
        zh: "引自证据；修改前请先查看来源。",
        ko: "근거에서 가져온 인용이에요. 바꾸기 전에 출처를 확인하세요.",
        ru: "Цитата из доказательства; перед изменением сверьтесь с источником." }
    /// Hint of a qualifier the reviewer wrote.
    qualifier_reviewer_hint { en: "Reviewer's statement · no verified source.",
        pt: "Declaração do revisor · sem fonte verificada.",
        es: "Declaración del revisor · sin fuente verificada.",
        fr: "Déclaration du relecteur · sans source vérifiée.",
        de: "Aussage der prüfenden Person · ohne verifizierte Quelle.",
        it: "Dichiarazione del revisore · senza fonte verificata.",
        ja: "レビュー担当者の記述 · 検証済みソースなし。",
        zh: "审阅者声明 · 无已验证来源。",
        ko: "검토자의 진술 · 검증된 출처 없음.",
        ru: "Заявление проверяющего · без подтверждённого источника." }
    /// Hint of an empty qualifier field.
    qualifier_add_hint { en: "Add a reviewer's statement (optional).",
        pt: "Adicionar declaração do revisor (opcional).",
        es: "Añadir una declaración del revisor (opcional).",
        fr: "Ajouter une déclaration du relecteur (facultatif).",
        de: "Aussage der prüfenden Person hinzufügen (optional).",
        it: "Aggiungi una dichiarazione del revisore (facoltativo).",
        ja: "レビュー担当者の記述を追加（任意）。",
        zh: "添加审阅者声明（可选）。",
        ko: "검토자의 진술 추가 (선택).",
        ru: "Добавить заявление проверяющего (необязательно)." }

    /// Error when the conversations of a grouped decision cannot be read.
    group_load_error { en: "Couldn't see which conversations this decision appeared in.",
        pt: "Não foi possível ver em quais conversas esta decisão apareceu.",
        es: "No se pudo ver en qué conversaciones apareció esta decisión.",
        fr: "Impossible de voir dans quelles conversations cette décision est apparue.",
        de: "Es ließ sich nicht ermitteln, in welchen Unterhaltungen diese Entscheidung vorkam.",
        it: "Impossibile vedere in quali conversazioni è comparsa questa decisione.",
        ja: "この決定がどの会話に現れたかを確認できませんでした。",
        zh: "无法查看该决策出现在哪些对话中。",
        ko: "이 결정이 어떤 대화에 나왔는지 확인하지 못했어요.",
        ru: "Не удалось узнать, в каких беседах появилось это решение." }
    /// Retry button.
    try_again { en: "Try again", pt: "Tentar novamente", es: "Reintentar",
        fr: "Réessayer", de: "Erneut versuchen", it: "Riprova", ja: "再試行",
        zh: "重试", ko: "다시 시도", ru: "Повторить" }
    /// Previous page of conversations.
    previous { en: "Previous", pt: "Anteriores", es: "Anteriores", fr: "Précédentes",
        de: "Vorherige", it: "Precedenti", ja: "前へ", zh: "上一页", ko: "이전",
        ru: "Предыдущие" }
    /// Next page of conversations.
    more_conversations { en: "More conversations", pt: "Mais conversas",
        es: "Más conversaciones", fr: "Plus de conversations",
        de: "Weitere Unterhaltungen", it: "Altre conversazioni",
        ja: "さらに会話を表示", zh: "更多对话", ko: "대화 더 보기", ru: "Ещё беседы" }
    /// Header of the section listing the conversations of a decision.
    also_appeared_in { en: "Also appeared in", pt: "Também apareceu em",
        es: "También apareció en", fr: "Est aussi apparue dans", de: "Kam auch vor in",
        it: "È comparsa anche in", ja: "次の会話にも登場", zh: "同样出现在",
        ko: "다음에도 나왔어요", ru: "Также появлялось в" }
    /// Rule of a decision found in several conversations.
    group_rule_hint { en: "Confirming or rejecting applies to all of them; each keeps its own evidence.",
        pt: "Confirmar ou rejeitar vale para todas; cada uma guarda sua evidência.",
        es: "Confirmar o rechazar vale para todas; cada una conserva su evidencia.",
        fr: "Confirmer ou rejeter vaut pour toutes ; chacune garde sa preuve.",
        de: "Bestätigen oder Ablehnen gilt für alle; jede behält ihren eigenen Beleg.",
        it: "Confermare o rifiutare vale per tutte; ognuna conserva la propria evidenza.",
        ja: "確定や却下はすべてに適用されます。それぞれが自分の根拠を保持します。",
        zh: "确认或拒绝对所有对话生效；每个对话保留各自的证据。",
        ko: "확인이나 거부는 모두에 적용되고, 각각 자기 근거를 유지해요.",
        ru: "Подтверждение или отклонение действует для всех; у каждой остаётся своё доказательство." }
    /// Toast: a discarded candidate is back in the queue.
    notice_back_to_queue { en: "Candidate back in the queue.",
        pt: "Candidato de volta à fila.", es: "Candidato de vuelta en la cola.",
        fr: "Candidat remis dans la file d’attente.",
        de: "Kandidat wieder in der Warteschlange.",
        it: "Candidato di nuovo in coda.", ja: "候補をキューに戻しました。",
        zh: "候选已放回队列。", ko: "후보를 대기열로 되돌렸어요.",
        ru: "Кандидат возвращён в очередь." }
    /// Toast: undoing did not work.
    notice_undo_failed { en: "Couldn't undo.", pt: "Não foi possível desfazer.",
        es: "No se pudo deshacer.", fr: "Impossible d’annuler.",
        de: "Rückgängig machen war nicht möglich.", it: "Impossibile annullare.",
        ja: "元に戻せませんでした。", zh: "无法撤销。", ko: "되돌리지 못했어요.",
        ru: "Не удалось отменить." }
}

formats! {
    /// Source description for a range of lines.
    source_lines(start: u64, end: u64) { en: "lines {start}–{end}",
        pt: "linhas {start}–{end}", es: "líneas {start}–{end}",
        fr: "lignes {start}–{end}", de: "Zeilen {start}–{end}",
        it: "righe {start}–{end}", ja: "{start}–{end} 行目", zh: "第 {start}–{end} 行",
        ko: "{start}–{end}행", ru: "строки {start}–{end}" }
    /// Source description for a code excerpt without line numbers.
    source_excerpt_lines(count: usize) { en: "{count} lines of the excerpt",
        pt: "{count} linhas do trecho", es: "{count} líneas del fragmento",
        fr: "{count} lignes de l’extrait", de: "{count} Zeilen des Ausschnitts",
        it: "{count} righe dell’estratto", ja: "抜粋の {count} 行",
        zh: "片段共 {count} 行", ko: "발췌 {count}행", ru: "{count} строк фрагмента" }
    /// Label of a conversation a decision appeared in.
    conversation(number: usize) { en: "Conversation {number}", pt: "Conversa {number}",
        es: "Conversación {number}", fr: "Conversation {number}",
        de: "Unterhaltung {number}", it: "Conversazione {number}",
        ja: "会話 {number}", zh: "对话 {number}", ko: "대화 {number}",
        ru: "Беседа {number}" }
    /// Label of a conversation whose candidate is already confirmed.
    conversation_confirmed(number: usize) { en: "Conversation {number} · already confirmed",
        pt: "Conversa {number} · já confirmada",
        es: "Conversación {number} · ya confirmada",
        fr: "Conversation {number} · déjà confirmée",
        de: "Unterhaltung {number} · bereits bestätigt",
        it: "Conversazione {number} · già confermata",
        ja: "会話 {number} · 確定済み", zh: "对话 {number} · 已确认",
        ko: "대화 {number} · 이미 확인함", ru: "Беседа {number} · уже подтверждена" }
    /// Screen-reader label of a button that opens one conversation's evidence.
    read_conversation_evidence(label: &str) { en: "Read the evidence of {label}",
        pt: "Ler a evidência da {label}", es: "Leer la evidencia de {label}",
        fr: "Lire la preuve de {label}", de: "Beleg von {label} lesen",
        it: "Leggi l’evidenza di {label}", ja: "{label} の根拠を読む",
        zh: "阅读{label}的证据", ko: "{label}의 근거 읽기",
        ru: "Прочитать доказательство: {label}" }
    /// Tooltip naming the capture behind a conversation.
    capture_tooltip(id: &str) { en: "Capture {id}", pt: "Captura {id}", es: "Captura {id}",
        fr: "Capture {id}", de: "Erfassung {id}", it: "Acquisizione {id}",
        ja: "キャプチャ {id}", zh: "捕获 {id}", ko: "캡처 {id}", ru: "Захват {id}" }
    /// Chip counting the conversations of a decision (always two or more).
    conversations_count(count: usize) { en: "{count} conversations",
        pt: "{count} conversas", es: "{count} conversaciones",
        fr: "{count} conversations", de: "{count} Unterhaltungen",
        it: "{count} conversazioni", ja: "{count} 件の会話", zh: "{count} 个对话",
        ko: "대화 {count}개", ru: "бесед: {count}" }
}

strings! {
    /// Review action: reject the candidate.
    review_reject { en: "Reject", pt: "Rejeitar", es: "Rechazar", fr: "Rejeter",
        de: "Ablehnen", it: "Rifiuta", ja: "却下", zh: "拒绝", ko: "거부", ru: "Отклонить" }
    /// Review action: bring a postponed candidate back.
    review_resume { en: "Resume", pt: "Retomar", es: "Reanudar", fr: "Reprendre",
        de: "Fortsetzen", it: "Riprendi", ja: "再開", zh: "恢复", ko: "다시 시작",
        ru: "Возобновить" }
    /// Review action: postpone the candidate.
    review_snooze { en: "Postpone", pt: "Adiar", es: "Aplazar", fr: "Reporter",
        de: "Zurückstellen", it: "Rimanda", ja: "保留", zh: "推迟", ko: "미루기",
        ru: "Отложить" }
    /// Review action: open the editor.
    review_adjust { en: "Adjust", pt: "Ajustar", es: "Ajustar", fr: "Ajuster",
        de: "Anpassen", it: "Modifica", ja: "調整", zh: "调整", ko: "조정",
        ru: "Изменить" }
    /// Review action: confirm the candidate.
    review_confirm { en: "Confirm", pt: "Confirmar", es: "Confirmar", fr: "Confirmer",
        de: "Bestätigen", it: "Conferma", ja: "確定", zh: "确认", ko: "확인",
        ru: "Подтвердить" }
    /// Footer while an action is being recorded.
    review_recording { en: "Recording…", pt: "Registrando…", es: "Registrando…",
        fr: "Enregistrement…", de: "Wird festgehalten …", it: "Registrazione…",
        ja: "記録中…", zh: "正在记录…", ko: "기록 중…", ru: "Запись…" }
    /// Kind of a candidate: a rule.
    kind_rule { en: "Rule", pt: "Regra", es: "Regla", fr: "Règle", de: "Regel",
        it: "Regola", ja: "ルール", zh: "规则", ko: "규칙", ru: "Правило" }
    /// Kind of a candidate: a decision.
    kind_decision { en: "Decision", pt: "Decisão", es: "Decisión", fr: "Décision",
        de: "Entscheidung", it: "Decisione", ja: "決定", zh: "决策", ko: "결정",
        ru: "Решение" }
    /// Tag on a row the automatic review left for the person.
    left_for_you_tag { en: "the AI left it for you", pt: "a IA deixou para você",
        es: "la IA lo dejó para ti", fr: "l’IA vous l’a laissé",
        de: "die KI hat es dir überlassen", it: "l’IA l’ha lasciato a te",
        ja: "AI があなたに任せました", zh: "AI 留给了你", ko: "AI가 맡겨 뒀어요",
        ru: "ИИ оставил это вам" }
    /// Mode switch: the person decides everything.
    mode_manual { en: "Manual", pt: "Manual", es: "Manual", fr: "Manuel",
        de: "Manuell", it: "Manuale", ja: "手動", zh: "手动", ko: "수동",
        ru: "Вручную" }
    /// Mode switch: the AI handles the cycle.
    mode_automatic { en: "Automatic", pt: "Automático", es: "Automático",
        fr: "Automatique", de: "Automatisch", it: "Automatico", ja: "自動", zh: "自动",
        ko: "자동", ru: "Автоматически" }
    /// What the automatic mode does with an AI provider.
    mode_judge_on { en: "The AI accepts what is safe and leaves the rest to you.",
        pt: "A IA aceita o que é seguro e deixa o resto para você.",
        es: "La IA acepta lo que es seguro y deja el resto para ti.",
        fr: "L’IA accepte ce qui est sûr et vous laisse le reste.",
        de: "Die KI übernimmt, was sicher ist, und überlässt dir den Rest.",
        it: "L’IA accetta ciò che è sicuro e lascia il resto a te.",
        ja: "安全なものは AI が受け入れ、残りはあなたに任せます。",
        zh: "AI 接受安全的内容，其余留给你。",
        ko: "안전한 것은 AI가 받아들이고, 나머지는 맡겨 드려요.",
        ru: "ИИ принимает то, что безопасно, а остальное оставляет вам." }
    /// What the automatic mode does without an AI provider.
    mode_judge_off { en: "No active AI provider: only local rules act.",
        pt: "Sem provedor de IA ativo: só as regras locais agem.",
        es: "Sin proveedor de IA activo: solo actúan las reglas locales.",
        fr: "Aucun fournisseur d’IA actif : seules les règles locales agissent.",
        de: "Kein KI-Anbieter aktiv: Nur die lokalen Regeln greifen.",
        it: "Nessun provider IA attivo: agiscono solo le regole locali.",
        ja: "有効な AI プロバイダーがありません。ローカルのルールだけが働きます。",
        zh: "没有启用的 AI 提供商：只有本地规则生效。",
        ko: "활성화된 AI 공급자가 없어서 로컬 규칙만 작동해요.",
        ru: "Нет активного ИИ-провайдера: действуют только локальные правила." }
    /// Screen-reader label of the toggle for the list of automatic work.
    ledger_toggle_aria { en: "Show what was done automatically",
        pt: "Mostrar o que foi feito sozinho", es: "Mostrar lo que se hizo solo",
        fr: "Afficher ce qui a été fait automatiquement",
        de: "Anzeigen, was automatisch erledigt wurde",
        it: "Mostra cosa è stato fatto in automatico",
        ja: "自動で処理された内容を表示", zh: "显示自动完成的内容",
        ko: "자동으로 처리된 내용 보기", ru: "Показать, что сделано автоматически" }
    /// Ledger item kind: a candidate.
    ledger_kind_candidate { en: "candidate", pt: "candidato", es: "candidato",
        fr: "candidat", de: "Kandidat", it: "candidato", ja: "候補", zh: "候选",
        ko: "후보", ru: "кандидат" }
    /// Ledger item kind: a relation.
    ledger_kind_relation { en: "relation", pt: "relação", es: "relación",
        fr: "relation", de: "Beziehung", it: "relazione", ja: "関係", zh: "关系",
        ko: "관계", ru: "связь" }
    /// Ledger item kind: a context claim.
    ledger_kind_claim { en: "context", pt: "contexto", es: "contexto", fr: "contexte",
        de: "Kontext", it: "contesto", ja: "コンテキスト", zh: "上下文",
        ko: "컨텍스트", ru: "контекст" }
    /// Ledger item kind: a link.
    ledger_kind_link { en: "link", pt: "ligação", es: "vínculo", fr: "lien",
        de: "Verknüpfung", it: "collegamento", ja: "リンク", zh: "关联", ko: "연결",
        ru: "ссылка" }
    /// Ledger verdict: accepted.
    ledger_verdict_accepted { en: "accepted", pt: "aceita", es: "Aceptado",
        fr: "Accepté", de: "akzeptiert", it: "Accettato", ja: "承認", zh: "接受",
        ko: "수락", ru: "Принято" }
    /// Ledger verdict: discarded.
    ledger_verdict_discarded { en: "discarded", pt: "descartada", es: "Descartado",
        fr: "Écarté", de: "verworfen", it: "Scartato", ja: "破棄", zh: "丢弃",
        ko: "폐기", ru: "Отброшено" }
    /// Ledger author: the local rules.
    ledger_by_rules { en: "by the rules", pt: "pelas regras", es: "por las reglas",
        fr: "par les règles", de: "durch die Regeln", it: "dalle regole", ja: "ルールが",
        zh: "规则", ko: "규칙이", ru: "правилами" }
    /// Ledger author: the AI.
    ledger_by_ai { en: "by the AI", pt: "pela IA", es: "por la IA", fr: "par l’IA",
        de: "durch die KI", it: "dall’IA", ja: "AI が", zh: "AI", ko: "AI가",
        ru: "ИИ" }
    /// Undo button.
    undo { en: "Undo", pt: "Desfazer", es: "Deshacer", fr: "Annuler",
        de: "Rückgängig", it: "Annulla", ja: "元に戻す", zh: "撤销", ko: "되돌리기",
        ru: "Отменить" }
    /// Dismiss button of a line or an error.
    dismiss { en: "Dismiss", pt: "Dispensar", es: "Descartar", fr: "Ignorer",
        de: "Ausblenden", it: "Ignora", ja: "閉じる", zh: "忽略", ko: "닫기",
        ru: "Скрыть" }
    /// Button hiding the low-relevance candidates again.
    low_hide { en: "Hide the low-relevance ones", pt: "Esconder os de baixa relevância",
        es: "Ocultar los de baja relevancia", fr: "Masquer ceux de faible pertinence",
        de: "Die mit geringer Relevanz ausblenden",
        it: "Nascondi quelli di bassa rilevanza", ja: "関連度の低いものを隠す",
        zh: "隐藏低相关度的项目", ko: "관련도 낮은 항목 숨기기",
        ru: "Скрыть малорелевантные" }
    /// Heading of the map section of a candidate.
    map_title { en: "On the map", pt: "No mapa", es: "En el mapa", fr: "Sur la carte",
        de: "Auf der Karte", it: "Sulla mappa", ja: "マップ上", zh: "在地图中",
        ko: "맵에서", ru: "На карте" }
    /// Tie verb: the entity uses it.
    link_uses { en: "uses", pt: "usa", es: "usa", fr: "utilise", de: "nutzt",
        it: "usa", ja: "使用", zh: "使用", ko: "사용", ru: "использует" }
    /// Tie verb: the entity applies to it.
    link_applies_to { en: "applies to", pt: "vale para", es: "vale para",
        fr: "s’applique à", de: "gilt für", it: "vale per", ja: "適用先", zh: "适用于",
        ko: "적용 대상", ru: "применяется к" }
    /// Tie verb: the entity changes it.
    link_changes { en: "changes", pt: "muda", es: "cambia", fr: "modifie",
        de: "ändert", it: "modifica", ja: "変更", zh: "更改", ko: "변경",
        ru: "меняет" }
    /// Hint above the map ties of a rule.
    map_rule_hint { en: "When you confirm, the rule starts to apply to the checked items.",
        pt: "Ao confirmar, a regra passa a valer nos itens marcados.",
        es: "Al confirmar, la regla pasa a valer en los elementos marcados.",
        fr: "À la confirmation, la règle s’applique aux éléments cochés.",
        de: "Beim Bestätigen gilt die Regel für die markierten Einträge.",
        it: "Alla conferma, la regola vale per gli elementi selezionati.",
        ja: "確定すると、ルールはチェックした項目に適用されます。",
        zh: "确认后，该规则将对已勾选的项目生效。",
        ko: "확인하면 규칙이 선택한 항목에 적용돼요.",
        ru: "После подтверждения правило начнёт действовать для отмеченных элементов." }
    /// Hint above the map ties of a decision.
    map_decision_hint { en: "When you confirm, the decision is linked to the checked items; unchecked ones do not come back as suggestions.",
        pt: "Ao confirmar, a decisão fica ligada aos itens marcados; os desmarcados não voltam como sugestão.",
        es: "Al confirmar, la decisión queda vinculada a los elementos marcados; los desmarcados no vuelven como sugerencia.",
        fr: "À la confirmation, la décision est liée aux éléments cochés ; ceux décochés ne reviennent pas en suggestion.",
        de: "Beim Bestätigen wird die Entscheidung mit den markierten Einträgen verknüpft; abgewählte kommen nicht als Vorschlag zurück.",
        it: "Alla conferma, la decisione viene collegata agli elementi selezionati; quelli deselezionati non tornano come suggerimento.",
        ja: "確定すると、決定はチェックした項目に結び付きます。チェックを外したものは提案として戻りません。",
        zh: "确认后，该决策将关联到已勾选的项目；未勾选的不会再作为建议出现。",
        ko: "확인하면 결정이 선택한 항목에 연결되고, 선택 해제한 항목은 제안으로 다시 나오지 않아요.",
        ru: "После подтверждения решение связывается с отмеченными элементами; снятые не вернутся как предложения." }
    /// Capture pill: waiting for analysis.
    capture_state_queued { en: "Waiting", pt: "Aguardando", es: "En espera", fr: "En attente", de: "Wartet", it: "In attesa", ja: "待機中", zh: "等待中", ko: "대기 중", ru: "Ожидает" }
    /// Capture pill: being analysed.
    capture_state_running { en: "Analysing", pt: "Analisando", es: "Analizando", fr: "Analyse en cours", de: "Wird analysiert", it: "In analisi", ja: "分析中", zh: "分析中", ko: "분석 중", ru: "Анализ" }
    /// Capture pill: analysis complete.
    capture_state_done { en: "Analysed", pt: "Analisada", es: "Analizada", fr: "Analysée", de: "Analysiert", it: "Analizzata", ja: "分析済み", zh: "已分析", ko: "분석됨", ru: "Проанализирован" }
    /// Capture pill: analysis failed.
    capture_state_failed { en: "Failed", pt: "Falhou", es: "Falló", fr: "Échec", de: "Fehlgeschlagen", it: "Non riuscita", ja: "失敗", zh: "失败", ko: "실패", ru: "Ошибка" }
    /// Capture pill: analysis not run.
    capture_state_skipped { en: "Skipped", pt: "Ignorada", es: "Omitida", fr: "Ignorée", de: "Übersprungen", it: "Saltata", ja: "スキップ", zh: "已跳过", ko: "건너뜀", ru: "Пропущен" }
    /// Capture pill: analysis cancelled.
    capture_state_cancelled { en: "Cancelled", pt: "Cancelada", es: "Cancelada", fr: "Annulée", de: "Abgebrochen", it: "Annullata", ja: "キャンセル", zh: "已取消", ko: "취소됨", ru: "Отменён" }
    /// Capture pill: received, state not reported.
    capture_state_received { en: "Received", pt: "Recebida", es: "Recibida", fr: "Reçue", de: "Empfangen", it: "Ricevuta", ja: "受信済み", zh: "已收到", ko: "수신됨", ru: "Получен" }
    /// Title of a capture without a user prompt.
    capture_untitled { en: "Capture without a prompt", pt: "Captura sem prompt", es: "Captura sin prompt", fr: "Capture sans prompt", de: "Erfassung ohne Prompt", it: "Acquisizione senza prompt", ja: "プロンプトのないキャプチャ", zh: "没有提示词的捕获", ko: "프롬프트 없는 캡처", ru: "Захват без запроса" }
    /// Button of the empty review queue that opens the captures in Diagnostics.
    captures_open { en: "View captures", pt: "Ver capturas", es: "Ver capturas",
        fr: "Voir les captures", de: "Erfassungen ansehen", it: "Vedi acquisizioni",
        ja: "キャプチャを見る", zh: "查看捕获", ko: "캡처 보기", ru: "Показать захваты" }
    /// Screen-reader label of the button that opens the captures in Diagnostics.
    captures_open_aria { en: "View captures in Diagnostics",
        pt: "Ver capturas em Diagnóstico", es: "Ver capturas en Diagnóstico",
        fr: "Voir les captures dans Diagnostic", de: "Erfassungen in der Diagnose ansehen",
        it: "Vedi le acquisizioni in Diagnostica", ja: "診断でキャプチャを見る",
        zh: "在诊断中查看捕获", ko: "진단에서 캡처 보기",
        ru: "Показать захваты в диагностике" }
    /// Eyebrow of the empty review queue.
    empty_eyebrow { en: "Review queue", pt: "Fila de revisão", es: "Cola de revisión",
        fr: "File d’attente de revue", de: "Prüfwarteschlange", it: "Coda di revisione",
        ja: "レビューキュー", zh: "审阅队列", ko: "검토 대기열",
        ru: "Очередь проверки" }
    /// Title of the empty review queue.
    empty_title { en: "Nothing waiting for review", pt: "Nada aguardando revisão",
        es: "Nada pendiente de revisión", fr: "Rien en attente de revue",
        de: "Nichts wartet auf Prüfung", it: "Niente in attesa di revisione",
        ja: "レビュー待ちはありません", zh: "没有等待审阅的内容",
        ko: "검토 대기 중인 항목이 없어요", ru: "Нет ничего на проверке" }
    /// What fills the review queue.
    empty_body { en: "When an OpenCode session records an engineering choice, the extractor proposes a candidate here for you to confirm, adjust or reject.",
        pt: "Quando uma sessão do OpenCode registrar uma escolha de engenharia, o extrator propõe um candidato aqui para você confirmar, ajustar ou rejeitar.",
        es: "Cuando una sesión de OpenCode registre una elección de ingeniería, el extractor propone aquí un candidato para que lo confirmes, ajustes o rechaces.",
        fr: "Quand une session OpenCode enregistre un choix d’ingénierie, l’extracteur propose ici un candidat que vous pouvez confirmer, ajuster ou rejeter.",
        de: "Wenn eine OpenCode-Sitzung eine Entwicklungsentscheidung festhält, schlägt der Extraktor hier einen Kandidaten vor, den du bestätigen, anpassen oder ablehnen kannst.",
        it: "Quando una sessione OpenCode registra una scelta di ingegneria, l’estrattore propone qui un candidato che puoi confermare, modificare o rifiutare.",
        ja: "OpenCode のセッションがエンジニアリング上の選択を記録すると、抽出器がここに候補を提案します。確定、調整、却下を選べます。",
        zh: "当 OpenCode 会话记录了一项工程选择时，提取器会在这里提出候选，供你确认、调整或拒绝。",
        ko: "OpenCode 세션이 엔지니어링 선택을 기록하면, 추출기가 여기에 후보를 제안해요. 확인하거나 조정하거나 거부할 수 있어요.",
        ru: "Когда сессия OpenCode записывает инженерный выбор, экстрактор предлагает здесь кандидата, которого вы можете подтвердить, скорректировать или отклонить." }
    /// Placeholder of the reading pane without a selection.
    select_candidate { en: "Select a candidate to read the evidence.",
        pt: "Selecione um candidato para ler as evidências.",
        es: "Selecciona un candidato para leer las evidencias.",
        fr: "Sélectionnez un candidat pour lire les preuves.",
        de: "Wähle einen Kandidaten, um die Belege zu lesen.",
        it: "Seleziona un candidato per leggere le evidenze.",
        ja: "候補を選ぶと、根拠を読めます。", zh: "选择一个候选以阅读证据。",
        ko: "후보를 선택하면 근거를 읽을 수 있어요.",
        ru: "Выберите кандидата, чтобы прочитать доказательства." }
    /// Error when a source of a conversation could not be read.
    source_read_error { en: "Couldn't read this source. Select it again to retry.",
        pt: "Não foi possível ler esta fonte. Selecione-a novamente para tentar.",
        es: "No se pudo leer esta fuente. Vuelve a seleccionarla para reintentar.",
        fr: "Impossible de lire cette source. Sélectionnez-la à nouveau pour réessayer.",
        de: "Diese Quelle ließ sich nicht lesen. Wähle sie erneut aus, um es noch einmal zu versuchen.",
        it: "Impossibile leggere questa fonte. Selezionala di nuovo per riprovare.",
        ja: "このソースを読み込めませんでした。もう一度選ぶと再試行します。",
        zh: "无法读取此来源。请再次选择以重试。",
        ko: "이 출처를 읽지 못했어요. 다시 선택하면 재시도해요.",
        ru: "Не удалось прочитать этот источник. Выберите его снова, чтобы повторить." }
    /// A candidate without any source.
    no_source { en: "No source available for this candidate.",
        pt: "Nenhuma fonte disponível para este candidato.",
        es: "Ninguna fuente disponible para este candidato.",
        fr: "Aucune source disponible pour ce candidat.",
        de: "Keine Quelle für diesen Kandidaten verfügbar.",
        it: "Nessuna fonte disponibile per questo candidato.",
        ja: "この候補に使えるソースはありません。", zh: "此候选没有可用的来源。",
        ko: "이 후보에 사용할 수 있는 출처가 없어요.",
        ru: "Для этого кандидата нет доступных источников." }
    /// Heading of the evidence section.
    evidence_title { en: "Evidence", pt: "Evidências", es: "Evidencias", fr: "Preuves",
        de: "Belege", it: "Evidenze", ja: "根拠", zh: "证据", ko: "근거",
        ru: "Доказательства" }
    /// Note above the evidence of another conversation.
    member_evidence_note { en: "Evidence from another conversation where the same decision appeared. Confirming applies to all of them.",
        pt: "Evidência de outra conversa em que a mesma decisão apareceu. Confirmar vale para todas.",
        es: "Evidencia de otra conversación en la que apareció la misma decisión. Confirmar vale para todas.",
        fr: "Preuve d’une autre conversation où la même décision est apparue. Confirmer vaut pour toutes.",
        de: "Beleg aus einer anderen Unterhaltung, in der dieselbe Entscheidung vorkam. Bestätigen gilt für alle.",
        it: "Evidenza di un’altra conversazione in cui è comparsa la stessa decisione. Confermare vale per tutte.",
        ja: "同じ決定が現れた別の会話の根拠です。確定はすべてに適用されます。",
        zh: "来自另一个出现相同决策的对话的证据。确认对所有对话生效。",
        ko: "같은 결정이 나온 다른 대화의 근거예요. 확인은 모두에 적용돼요.",
        ru: "Доказательство из другой беседы, где появилось то же решение. Подтверждение действует для всех." }
    /// Heading of where one conversation came from.
    member_origin_title { en: "Origin of this conversation", pt: "Origem desta conversa",
        es: "Origen de esta conversación", fr: "Origine de cette conversation",
        de: "Herkunft dieser Unterhaltung", it: "Origine di questa conversazione",
        ja: "この会話の出どころ", zh: "此对话的来源", ko: "이 대화의 출처",
        ru: "Источник этой беседы" }
    /// Heading of where the candidate came from.
    origin_title { en: "Origin", pt: "Origem", es: "Origen", fr: "Origine",
        de: "Herkunft", it: "Origine", ja: "出どころ", zh: "来源", ko: "출처",
        ru: "Источник" }
    /// Heading of the extractor's confidence.
    confidence_title { en: "Extraction confidence", pt: "Confiança da extração",
        es: "Confianza de la extracción", fr: "Confiance de l’extraction",
        de: "Konfidenz der Extraktion", it: "Affidabilità dell’estrazione",
        ja: "抽出の信頼度", zh: "提取置信度", ko: "추출 신뢰도",
        ru: "Уверенность извлечения" }
    /// Caption of the confidence meter: it is only an estimate.
    confidence_estimate { en: "the extractor's estimate", pt: "estimativa do extrator",
        es: "estimación del extractor", fr: "estimation de l’extracteur",
        de: "Schätzung des Extraktors", it: "stima dell’estrattore",
        ja: "抽出器の推定値", zh: "提取器的估计值", ko: "추출기의 추정치",
        ru: "оценка экстрактора" }
    /// Criterion: affects several parts of the project.
    criterion_cross_cutting { en: "affects several parts", pt: "afeta várias partes",
        es: "afecta a varias partes", fr: "touche plusieurs parties",
        de: "betrifft mehrere Teile", it: "riguarda più parti",
        ja: "複数のパートに影響", zh: "影响多个部分", ko: "여러 파트에 영향",
        ru: "затрагивает несколько частей" }
    /// Criterion: data or contract.
    criterion_data_or_contract { en: "data or contract", pt: "dados ou contrato",
        es: "datos o contrato", fr: "données ou contrat", de: "Daten oder Vertrag",
        it: "dati o contratto", ja: "データまたは契約", zh: "数据或契约",
        ko: "데이터 또는 계약", ru: "данные или контракт" }
    /// Criterion: security or privacy.
    criterion_security_or_privacy { en: "security or privacy",
        pt: "segurança ou privacidade", es: "seguridad o privacidad",
        fr: "sécurité ou confidentialité", de: "Sicherheit oder Datenschutz",
        it: "sicurezza o privacy", ja: "セキュリティまたはプライバシー",
        zh: "安全或隐私", ko: "보안 또는 개인정보",
        ru: "безопасность или конфиденциальность" }
    /// Criterion: external dependency.
    criterion_external_dependency { en: "external dependency",
        pt: "dependência externa", es: "dependencia externa",
        fr: "dépendance externe", de: "externe Abhängigkeit",
        it: "dipendenza esterna", ja: "外部依存", zh: "外部依赖",
        ko: "외부 의존성", ru: "внешняя зависимость" }
    /// Criterion: hard to reverse.
    criterion_hard_to_reverse { en: "hard to reverse", pt: "difícil de reverter",
        es: "difícil de revertir", fr: "difficile à annuler",
        de: "schwer rückgängig zu machen", it: "difficile da annullare",
        ja: "元に戻しにくい", zh: "难以回退", ko: "되돌리기 어려움",
        ru: "трудно откатить" }
    /// Criterion: first of its kind in the project.
    criterion_first_of_a_kind { en: "first of its kind in the project",
        pt: "primeira vez no projeto", es: "primera vez en el proyecto",
        fr: "première fois dans le projet", de: "erstmals im Projekt",
        it: "prima volta nel progetto", ja: "プロジェクトで初めて",
        zh: "项目中的首次", ko: "프로젝트에서 처음", ru: "впервые в проекте" }
    /// Criterion: solves a recurring problem.
    criterion_past_problem { en: "solves a recurring problem",
        pt: "resolve problema recorrente", es: "resuelve un problema recurrente",
        fr: "résout un problème récurrent", de: "löst ein wiederkehrendes Problem",
        it: "risolve un problema ricorrente", ja: "繰り返し起きる問題を解決",
        zh: "解决反复出现的问题", ko: "반복되는 문제를 해결",
        ru: "решает повторяющуюся проблему" }
    /// Criterion: constrains future work.
    criterion_constrains_future_work { en: "constrains future work",
        pt: "condiciona trabalho futuro", es: "condiciona el trabajo futuro",
        fr: "conditionne le travail futur", de: "schränkt künftige Arbeit ein",
        it: "condiziona il lavoro futuro", ja: "今後の作業に制約を与える",
        zh: "制约未来工作", ko: "향후 작업을 제약", ru: "ограничивает будущую работу" }
    /// Criterion: anything else.
    criterion_other { en: "another reason", pt: "outro motivo", es: "otro motivo",
        fr: "autre raison", de: "anderer Grund", it: "altro motivo", ja: "その他の理由",
        zh: "其他原因", ko: "다른 이유", ru: "другая причина" }
    /// Reason title of the AI's written reason.
    reason_title { en: "Reason written by the AI", pt: "Motivo escrito pela IA",
        es: "Motivo escrito por la IA", fr: "Raison rédigée par l’IA",
        de: "Von der KI geschriebener Grund", it: "Motivo scritto dall’IA",
        ja: "AI が書いた理由", zh: "AI 写的原因", ko: "AI가 쓴 이유",
        ru: "Причина, написанная ИИ" }
    /// Screen-reader label to hide the AI's reason.
    reason_hide_aria { en: "Hide the reason written by the AI",
        pt: "Esconder o motivo escrito pela IA",
        es: "Ocultar el motivo escrito por la IA",
        fr: "Masquer la raison rédigée par l’IA",
        de: "Den von der KI geschriebenen Grund ausblenden",
        it: "Nascondi il motivo scritto dall’IA", ja: "AI が書いた理由を隠す",
        zh: "隐藏 AI 写的原因", ko: "AI가 쓴 이유 숨기기",
        ru: "Скрыть причину, написанную ИИ" }
    /// Screen-reader label to show the AI's reason.
    reason_show_aria { en: "Show the reason written by the AI",
        pt: "Mostrar o motivo escrito pela IA",
        es: "Mostrar el motivo escrito por la IA",
        fr: "Afficher la raison rédigée par l’IA",
        de: "Den von der KI geschriebenen Grund anzeigen",
        it: "Mostra il motivo scritto dall’IA", ja: "AI が書いた理由を表示",
        zh: "显示 AI 写的原因", ko: "AI가 쓴 이유 보기",
        ru: "Показать причину, написанную ИИ" }
    /// Title of the queue panel.
    queue_title { en: "Waiting for review", pt: "Aguardando revisão",
        es: "Pendientes de revisión", fr: "En attente de revue",
        de: "Wartet auf Prüfung", it: "In attesa di revisione", ja: "レビュー待ち",
        zh: "等待审阅", ko: "검토 대기", ru: "Ожидают проверки" }
    /// Refresh button of the queue.
    refresh { en: "Refresh", pt: "Atualizar", es: "Actualizar", fr: "Actualiser",
        de: "Aktualisieren", it: "Aggiorna", ja: "更新", zh: "刷新", ko: "새로 고침",
        ru: "Обновить" }
    /// Queue message before the first load.
    queue_not_loaded { en: "Refresh to load the candidates.",
        pt: "Atualize para carregar os candidatos.",
        es: "Actualiza para cargar los candidatos.",
        fr: "Actualisez pour charger les candidats.",
        de: "Aktualisiere, um die Kandidaten zu laden.",
        it: "Aggiorna per caricare i candidati.",
        ja: "更新すると候補を読み込みます。", zh: "刷新以加载候选。",
        ko: "새로 고치면 후보를 불러와요.",
        ru: "Обновите, чтобы загрузить кандидатов." }
    /// Queue message when it is empty.
    queue_empty { en: "Empty queue.", pt: "Fila vazia.", es: "Cola vacía.",
        fr: "File d’attente vide.", de: "Warteschlange leer.", it: "Coda vuota.",
        ja: "キューは空です。", zh: "队列为空。", ko: "대기열이 비어 있어요.",
        ru: "Очередь пуста." }
    /// Queue message when the search matches nothing.
    queue_no_match { en: "No loaded candidate matches the search.",
        pt: "Nenhum candidato carregado corresponde à busca.",
        es: "Ningún candidato cargado coincide con la búsqueda.",
        fr: "Aucun candidat chargé ne correspond à la recherche.",
        de: "Kein geladener Kandidat passt zur Suche.",
        it: "Nessun candidato caricato corrisponde alla ricerca.",
        ja: "読み込み済みの候補に検索条件に合うものはありません。",
        zh: "没有已加载的候选符合搜索。",
        ko: "불러온 후보 중 검색과 일치하는 항목이 없어요.",
        ru: "Ни один загруженный кандидат не соответствует поиску." }
    /// Button loading the next page of the queue.
    load_more { en: "Load more", pt: "Carregar mais", es: "Cargar más",
        fr: "Charger plus", de: "Mehr laden", it: "Carica altri", ja: "さらに読み込む",
        zh: "加载更多", ko: "더 불러오기", ru: "Загрузить ещё" }
    /// Candidate status: waiting for review.
    status_pending { en: "Pending", pt: "Pendente", es: "Pendiente", fr: "En attente",
        de: "Ausstehend", it: "In sospeso", ja: "未処理", zh: "待处理", ko: "대기 중",
        ru: "Ожидает" }
    /// Candidate status: postponed.
    status_snoozed { en: "Postponed", pt: "Adiado", es: "Aplazado", fr: "Reporté",
        de: "Zurückgestellt", it: "Rimandato", ja: "保留中", zh: "已推迟", ko: "미룸",
        ru: "Отложено" }
    /// Candidate status: confirmed.
    status_accepted { en: "Confirmed", pt: "Confirmado", es: "Confirmado",
        fr: "Confirmé", de: "Bestätigt", it: "Confermato", ja: "確定済み",
        zh: "已确认", ko: "확인됨", ru: "Подтверждено" }
    /// Candidate status: adjusted, then confirmed.
    status_edited { en: "Adjusted and confirmed", pt: "Ajustado e confirmado",
        es: "Ajustado y confirmado", fr: "Ajusté et confirmé",
        de: "Angepasst und bestätigt", it: "Modificato e confermato",
        ja: "調整して確定済み", zh: "已调整并确认", ko: "조정 후 확인됨",
        ru: "Исправлено и подтверждено" }
    /// Candidate status: rejected.
    status_dismissed { en: "Rejected", pt: "Rejeitado", es: "Rechazado",
        fr: "Rejeté", de: "Abgelehnt", it: "Rifiutato", ja: "却下済み", zh: "已拒绝",
        ko: "거부됨", ru: "Отклонено" }
    /// Error: the edits did not pass validation.
    error_invalid_edits { en: "Fill in the question, the choice and the reason. Limits: 500, 1,000 and 4,000 characters, respectively.",
        pt: "Preencha pergunta, escolha e motivo. Limites: 500, 1.000 e 4.000 caracteres, respectivamente.",
        es: "Rellena la pregunta, la elección y el motivo. Límites: 500, 1.000 y 4.000 caracteres, respectivamente.",
        fr: "Renseignez la question, le choix et la raison. Limites : 500, 1 000 et 4 000 caractères, respectivement.",
        de: "Fülle Frage, Wahl und Grund aus. Limits: 500, 1.000 bzw. 4.000 Zeichen.",
        it: "Compila domanda, scelta e motivo. Limiti: 500, 1.000 e 4.000 caratteri, rispettivamente.",
        ja: "質問・選択・理由を入力してください。上限はそれぞれ 500、1,000、4,000 文字です。",
        zh: "请填写问题、选择和原因。上限分别为 500、1,000 和 4,000 个字符。",
        ko: "질문, 선택, 이유를 입력하세요. 한도는 각각 500자, 1,000자, 4,000자예요.",
        ru: "Заполните вопрос, выбор и причину. Лимиты: 500, 1 000 и 4 000 символов соответственно." }
    /// Error: the candidate changed under the person.
    error_candidate_changed { en: "This candidate changed. Refresh the Inbox to continue.",
        pt: "Este candidato mudou. Atualize a Inbox para continuar.",
        es: "Este candidato cambió. Actualiza la Inbox para continuar.",
        fr: "Ce candidat a changé. Actualisez la Inbox pour continuer.",
        de: "Dieser Kandidat hat sich geändert. Aktualisiere die Inbox, um fortzufahren.",
        it: "Questo candidato è cambiato. Aggiorna la Inbox per continuare.",
        ja: "この候補は変更されました。続けるには Inbox を更新してください。",
        zh: "此候选已更改。请刷新 Inbox 后继续。",
        ko: "이 후보가 바뀌었어요. 계속하려면 Inbox를 새로 고치세요.",
        ru: "Этот кандидат изменился. Обновите Inbox, чтобы продолжить." }
    /// Error: the inbox could not be loaded.
    error_load { en: "Couldn't load the Inbox. Try refreshing; if it persists, reopen the app.",
        pt: "Não foi possível carregar a Inbox. Tente atualizar; se persistir, reabra o app.",
        es: "No se pudo cargar la Inbox. Prueba a actualizar; si persiste, reabre la app.",
        fr: "Impossible de charger la Inbox. Essayez d’actualiser ; si le problème persiste, rouvrez l’app.",
        de: "Die Inbox ließ sich nicht laden. Versuche es mit Aktualisieren; falls es bleibt, öffne die App neu.",
        it: "Impossibile caricare la Inbox. Prova ad aggiornare; se persiste, riapri l’app.",
        ja: "Inbox を読み込めませんでした。更新してみて、直らなければアプリを開き直してください。",
        zh: "无法加载 Inbox。请尝试刷新；若仍有问题，请重新打开应用。",
        ko: "Inbox를 불러오지 못했어요. 새로 고쳐 보고, 계속되면 앱을 다시 열어 주세요.",
        ru: "Не удалось загрузить Inbox. Попробуйте обновить; если не помогает, перезапустите приложение." }
    /// Toast: changes saved, the candidate stays in review.
    notice_edits_saved { en: "Changes saved. The candidate stays in review.",
        pt: "Ajustes salvos. O candidato continua na revisão.",
        es: "Ajustes guardados. El candidato sigue en revisión.",
        fr: "Ajustements enregistrés. Le candidat reste en revue.",
        de: "Änderungen gespeichert. Der Kandidat bleibt in der Prüfung.",
        it: "Modifiche salvate. Il candidato resta in revisione.",
        ja: "調整を保存しました。候補はレビュー中のままです。",
        zh: "调整已保存。候选仍在审阅中。", ko: "조정을 저장했어요. 후보는 계속 검토 중이에요.",
        ru: "Правки сохранены. Кандидат остаётся на проверке." }
    /// Toast: changes confirmed, decision created and linked.
    notice_edits_confirmed_linked { en: "Changes confirmed. Decision created and linked to the map.",
        pt: "Ajustes confirmados. Decisão criada e ligada ao mapa.",
        es: "Ajustes confirmados. Decisión creada y vinculada al mapa.",
        fr: "Ajustements confirmés. Décision créée et liée à la carte.",
        de: "Änderungen bestätigt. Entscheidung erstellt und mit der Karte verknüpft.",
        it: "Modifiche confermate. Decisione creata e collegata alla mappa.",
        ja: "調整を確定しました。決定を作成し、マップに結び付けました。",
        zh: "调整已确认。已创建决策并关联到地图。",
        ko: "조정을 확인했어요. 결정을 만들고 맵에 연결했어요.",
        ru: "Правки подтверждены. Решение создано и связано с картой." }
    /// Toast: changes confirmed, decision created.
    notice_edits_confirmed { en: "Changes confirmed. Decision created.",
        pt: "Ajustes confirmados. Decisão criada.",
        es: "Ajustes confirmados. Decisión creada.",
        fr: "Ajustements confirmés. Décision créée.",
        de: "Änderungen bestätigt. Entscheidung erstellt.",
        it: "Modifiche confermate. Decisione creata.",
        ja: "調整を確定しました。決定を作成しました。",
        zh: "调整已确认。已创建决策。", ko: "조정을 확인했어요. 결정을 만들었어요.",
        ru: "Правки подтверждены. Решение создано." }
    /// Toast: candidate confirmed, decision created and linked.
    notice_confirmed_linked { en: "Candidate confirmed. Decision created and linked to the map.",
        pt: "Candidato confirmado. Decisão criada e ligada ao mapa.",
        es: "Candidato confirmado. Decisión creada y vinculada al mapa.",
        fr: "Candidat confirmé. Décision créée et liée à la carte.",
        de: "Kandidat bestätigt. Entscheidung erstellt und mit der Karte verknüpft.",
        it: "Candidato confermato. Decisione creata e collegata alla mappa.",
        ja: "候補を確定しました。決定を作成し、マップに結び付けました。",
        zh: "候选已确认。已创建决策并关联到地图。",
        ko: "후보를 확인했어요. 결정을 만들고 맵에 연결했어요.",
        ru: "Кандидат подтверждён. Решение создано и связано с картой." }
    /// Toast: candidate confirmed, decision created.
    notice_confirmed { en: "Candidate confirmed. Decision created.",
        pt: "Candidato confirmado. Decisão criada.",
        es: "Candidato confirmado. Decisión creada.",
        fr: "Candidat confirmé. Décision créée.",
        de: "Kandidat bestätigt. Entscheidung erstellt.",
        it: "Candidato confermato. Decisione creata.",
        ja: "候補を確定しました。決定を作成しました。",
        zh: "候选已确认。已创建决策。", ko: "후보를 확인했어요. 결정을 만들었어요.",
        ru: "Кандидат подтверждён. Решение создано." }
    /// Toast: candidate rejected.
    notice_rejected { en: "Candidate rejected.", pt: "Candidato rejeitado.",
        es: "Candidato rechazado.", fr: "Candidat rejeté.", de: "Kandidat abgelehnt.",
        it: "Candidato rifiutato.", ja: "候補を却下しました。", zh: "候选已拒绝。",
        ko: "후보를 거부했어요.", ru: "Кандидат отклонён." }
    /// Toast: a postponed candidate is back in review.
    notice_resumed { en: "Candidate back in review.",
        pt: "Candidato retomado para revisão.",
        es: "Candidato retomado para revisión.", fr: "Candidat repris en revue.",
        de: "Kandidat wieder in der Prüfung.", it: "Candidato ripreso per la revisione.",
        ja: "候補のレビューを再開しました。", zh: "候选已恢复审阅。",
        ko: "후보를 다시 검토로 돌렸어요.", ru: "Кандидат снова на проверке." }
    /// Toast: candidate postponed.
    notice_snoozed { en: "Candidate postponed. You can resume it later.",
        pt: "Candidato adiado. Você pode retomá-lo depois.",
        es: "Candidato aplazado. Puedes retomarlo después.",
        fr: "Candidat reporté. Vous pourrez le reprendre plus tard.",
        de: "Kandidat zurückgestellt. Du kannst ihn später wieder aufnehmen.",
        it: "Candidato rimandato. Puoi riprenderlo più tardi.",
        ja: "候補を保留にしました。あとで再開できます。",
        zh: "候选已推迟。你可以稍后恢复。",
        ko: "후보를 미뤘어요. 나중에 다시 시작할 수 있어요.",
        ru: "Кандидат отложен. Вы можете вернуться к нему позже." }
    /// Toast: a streak of quick confirmations.
    notice_streak { en: "High pace: open the evidence of one of the next ones before confirming.",
        pt: "Ritmo alto: abra a evidência de um dos próximos antes de confirmar.",
        es: "Ritmo alto: abre la evidencia de uno de los siguientes antes de confirmar.",
        fr: "Rythme élevé : ouvrez la preuve de l’un des suivants avant de confirmer.",
        de: "Hohes Tempo: Öffne den Beleg eines der nächsten, bevor du bestätigst.",
        it: "Ritmo alto: apri l’evidenza di uno dei prossimi prima di confermare.",
        ja: "ペースが速めです。確定する前に、次のどれかの根拠を開いてみてください。",
        zh: "节奏较快：确认之前，请先打开接下来某一项的证据。",
        ko: "속도가 빨라요. 확인하기 전에 다음 후보 중 하나의 근거를 열어 보세요.",
        ru: "Высокий темп: перед подтверждением откройте доказательство одного из следующих." }
    /// Toast: a rejection was undone.
    notice_reject_undone { en: "Rejection undone.", pt: "Rejeição desfeita.",
        es: "Rechazo deshecho.", fr: "Rejet annulé.",
        de: "Ablehnung rückgängig gemacht.", it: "Rifiuto annullato.",
        ja: "却下を取り消しました。", zh: "已撤销拒绝。", ko: "거부를 되돌렸어요.",
        ru: "Отклонение отменено." }
    /// Toast: a postponement was undone.
    notice_snooze_undone { en: "Postponement undone.", pt: "Adiamento desfeito.",
        es: "Aplazamiento deshecho.", fr: "Report annulé.",
        de: "Zurückstellung rückgängig gemacht.", it: "Rinvio annullato.",
        ja: "保留を取り消しました。", zh: "已撤销推迟。", ko: "미루기를 되돌렸어요.",
        ru: "Откладывание отменено." }
    /// Toast: the decision was saved but the map could not be updated.
    notice_map_failed { en: "Confirmation saved. Couldn't update the map; open the Map to review.",
        pt: "Confirmação salva. Não foi possível atualizar o mapa; abra o Mapa para revisar.",
        es: "Confirmación guardada. No se pudo actualizar el mapa; abre el Mapa para revisar.",
        fr: "Confirmation enregistrée. Impossible de mettre à jour la carte ; ouvrez la Carte pour vérifier.",
        de: "Bestätigung gespeichert. Die Karte ließ sich nicht aktualisieren; öffne die Karte zur Kontrolle.",
        it: "Conferma salvata. Impossibile aggiornare la mappa; apri la Mappa per verificare.",
        ja: "確定を保存しました。マップを更新できませんでした。マップを開いて確認してください。",
        zh: "确认已保存。无法更新地图；请打开地图查看。",
        ko: "확인을 저장했어요. 맵을 업데이트하지 못했어요. 맵을 열어서 확인하세요.",
        ru: "Подтверждение сохранено. Не удалось обновить карту; откройте Карту, чтобы проверить." }
    /// Toast: rule created and linked.
    notice_rule_linked { en: "Rule created and linked to the map.",
        pt: "Regra criada e ligada ao mapa.", es: "Regla creada y vinculada al mapa.",
        fr: "Règle créée et liée à la carte.",
        de: "Regel erstellt und mit der Karte verknüpft.",
        it: "Regola creata e collegata alla mappa.",
        ja: "ルールを作成し、マップに結び付けました。", zh: "已创建规则并关联到地图。",
        ko: "규칙을 만들고 맵에 연결했어요.", ru: "Правило создано и связано с картой." }
    /// Toast: rule created.
    notice_rule { en: "Rule created.", pt: "Regra criada.", es: "Regla creada.",
        fr: "Règle créée.", de: "Regel erstellt.", it: "Regola creata.",
        ja: "ルールを作成しました。", zh: "已创建规则。", ko: "규칙을 만들었어요.",
        ru: "Правило создано." }
    /// Toast: decision created and linked.
    notice_decision_linked { en: "Decision created and linked to the map.",
        pt: "Decisão criada e ligada ao mapa.",
        es: "Decisión creada y vinculada al mapa.",
        fr: "Décision créée et liée à la carte.",
        de: "Entscheidung erstellt und mit der Karte verknüpft.",
        it: "Decisione creata e collegata alla mappa.",
        ja: "決定を作成し、マップに結び付けました。", zh: "已创建决策并关联到地图。",
        ko: "결정을 만들고 맵에 연결했어요.", ru: "Решение создано и связано с картой." }
    /// Toast: decision created.
    notice_decision { en: "Decision created.", pt: "Decisão criada.",
        es: "Decisión creada.", fr: "Décision créée.", de: "Entscheidung erstellt.",
        it: "Decisione creata.", ja: "決定を作成しました。", zh: "已创建决策。",
        ko: "결정을 만들었어요.", ru: "Решение создано." }
}

formats! {
    /// Screen-reader label of one candidate row.
    row_aria(kind: &str, question: &str, significance: f32, confidence: f32) {
        en: "{kind}: {question}. Relevance {significance:.0}%, confidence {confidence:.0}%.",
        pt: "{kind}: {question}. Relevância {significance:.0}%, confiança {confidence:.0}%.",
        es: "{kind}: {question}. Relevancia {significance:.0}%, confianza {confidence:.0}%.",
        fr: "{kind} : {question}. Pertinence {significance:.0} %, confiance {confidence:.0} %.",
        de: "{kind}: {question}. Relevanz {significance:.0} %, Konfidenz {confidence:.0} %.",
        it: "{kind}: {question}. Rilevanza {significance:.0}%, affidabilità {confidence:.0}%.",
        ja: "{kind}: {question}。関連度 {significance:.0}%、信頼度 {confidence:.0}%。",
        zh: "{kind}：{question}。相关度 {significance:.0}%，置信度 {confidence:.0}%。",
        ko: "{kind}: {question}. 관련도 {significance:.0}%, 신뢰도 {confidence:.0}%.",
        ru: "{kind}: {question}. Релевантность {significance:.0}%, уверенность {confidence:.0}%." }
    /// Heading of the list of what the review did on its own.
    ledger_title(count: usize) { en: "Done automatically · {count}",
        pt: "Feito sozinho · {count}", es: "Hecho solo · {count}",
        fr: "Fait automatiquement · {count}", de: "Automatisch erledigt · {count}",
        it: "Fatto in automatico · {count}", ja: "自動処理 · {count}",
        zh: "自动完成 · {count}", ko: "자동 처리 · {count}",
        ru: "Сделано автоматически · {count}" }
    /// One line of what the review did on its own.
    ledger_line(kind: &str, verdict: &str, by: &str, time: &str, reason: &str) {
        en: "{kind} {verdict} {by} · {time} · {reason}",
        pt: "{kind} {verdict} {by}, há {time} · {reason}",
        es: "{verdict} {by}: {kind} · {time} · {reason}",
        fr: "{verdict} {by} : {kind} · {time} · {reason}",
        de: "{kind} {verdict} {by} · {time} · {reason}",
        it: "{verdict} {by}: {kind} · {time} · {reason}",
        ja: "{by}{kind}を{verdict} · {time} · {reason}",
        zh: "{by}{verdict}了{kind} · {time} · {reason}",
        ko: "{by} {kind} {verdict} · {time} · {reason}",
        ru: "{verdict} {by}: {kind} · {time} · {reason}" }
    /// Screen-reader label of an accepted ledger line that opens its decision.
    open_decision_aria(title: &str) { en: "Open the decision: {title}",
        pt: "Abrir a decisão: {title}", es: "Abrir la decisión: {title}",
        fr: "Ouvrir la décision : {title}", de: "Entscheidung öffnen: {title}",
        it: "Apri la decisione: {title}", ja: "決定を開く: {title}",
        zh: "打开决策：{title}", ko: "결정 열기: {title}",
        ru: "Открыть решение: {title}" }
    /// The briefing line: what changed since the previous visit.
    briefing_line(time: &str, parts: &str) {
        en: "Since the last visit ({time}): {parts}",
        pt: "Desde a última visita, há {time}: {parts}",
        es: "Desde la última visita ({time}): {parts}",
        fr: "Depuis la dernière visite ({time}) : {parts}",
        de: "Seit dem letzten Besuch ({time}): {parts}",
        it: "Dall’ultima visita ({time}): {parts}",
        ja: "前回の訪問以降（{time}）: {parts}",
        zh: "自上次访问（{time}）以来：{parts}",
        ko: "지난 방문 이후({time}): {parts}",
        ru: "С прошлого визита ({time}): {parts}" }
    /// Button showing the low-relevance candidates kept out of the queue.
    low_show(count: usize) { en: "Show {count} low-relevance",
        pt: "Mostrar {count} de baixa relevância",
        es: "Mostrar {count} de baja relevancia",
        fr: "Afficher {count} de faible pertinence",
        de: "{count} mit geringer Relevanz anzeigen",
        it: "Mostra {count} di bassa rilevanza",
        ja: "関連度の低い {count} 件を表示", zh: "显示 {count} 个低相关度项目",
        ko: "관련도 낮은 {count}개 보기", ru: "Показать малорелевантные: {count}" }
    /// A count out of a total: kept ties, visible rows.
    count_of(count: usize, total: usize) { en: "{count} of {total}",
        pt: "{count} de {total}", es: "{count} de {total}", fr: "{count} sur {total}",
        de: "{count} von {total}", it: "{count} su {total}", ja: "{count}/{total}",
        zh: "{count}/{total}", ko: "{count}/{total}", ru: "{count} из {total}" }
    /// Files no component of the map covers yet.
    map_uncovered(shown: &str) {
        en: "No component on the map: {shown}. Create it on the Map to link next time.",
        pt: "Sem componente no mapa: {shown}. Crie no Mapa para ligar da próxima vez.",
        es: "Sin componente en el mapa: {shown}. Créalo en el Mapa para vincular la próxima vez.",
        fr: "Aucun composant sur la carte : {shown}. Créez-le dans la Carte pour le lier la prochaine fois.",
        de: "Keine Komponente auf der Karte: {shown}. Lege sie in der Karte an, um beim nächsten Mal zu verknüpfen.",
        it: "Nessun componente sulla mappa: {shown}. Crealo nella Mappa per collegarlo la prossima volta.",
        ja: "マップにコンポーネントがありません: {shown}。次回リンクできるよう、マップで作成してください。",
        zh: "地图上没有对应组件：{shown}。请在地图中创建，下次即可关联。",
        ko: "맵에 컴포넌트가 없어요: {shown}. 맵에서 만들면 다음에 연결할 수 있어요.",
        ru: "На карте нет компонента: {shown}. Создайте его на Карте, чтобы связать в следующий раз." }
    /// Same, when more files than the ones shown are uncovered.
    map_uncovered_more(shown: &str, more: usize) {
        en: "No component on the map: {shown} and {more} more. Create it on the Map to link next time.",
        pt: "Sem componente no mapa: {shown} e mais {more}. Crie no Mapa para ligar da próxima vez.",
        es: "Sin componente en el mapa: {shown} y {more} más. Créalo en el Mapa para vincular la próxima vez.",
        fr: "Aucun composant sur la carte : {shown} et {more} de plus. Créez-le dans la Carte pour le lier la prochaine fois.",
        de: "Keine Komponente auf der Karte: {shown} und {more} weitere. Lege sie in der Karte an, um beim nächsten Mal zu verknüpfen.",
        it: "Nessun componente sulla mappa: {shown} e altri {more}. Crealo nella Mappa per collegarlo la prossima volta.",
        ja: "マップにコンポーネントがありません: {shown} ほか {more} 件。次回リンクできるよう、マップで作成してください。",
        zh: "地图上没有对应组件：{shown} 等另外 {more} 个。请在地图中创建，下次即可关联。",
        ko: "맵에 컴포넌트가 없어요: {shown} 외 {more}개. 맵에서 만들면 다음에 연결할 수 있어요.",
        ru: "На карте нет компонента: {shown} и ещё {more}. Создайте его на Карте, чтобы связать в следующий раз." }
    /// Line of why a candidate matters.
    why_it_matters(criteria: &str) { en: "Why it matters: {criteria}",
        pt: "Por que importa: {criteria}", es: "Por qué importa: {criteria}",
        fr: "Pourquoi c’est important : {criteria}", de: "Warum es wichtig ist: {criteria}",
        it: "Perché è importante: {criteria}", ja: "重要な理由: {criteria}",
        zh: "重要的原因：{criteria}", ko: "중요한 이유: {criteria}",
        ru: "Почему это важно: {criteria}" }
    /// Why the review left a candidate for the person.
    left_for_you_reason(reason: &str) { en: "The AI left this one for you: {reason}",
        pt: "A IA deixou este para você: {reason}",
        es: "La IA dejó este para ti: {reason}",
        fr: "L’IA vous a laissé celui-ci : {reason}",
        de: "Die KI hat dir diesen überlassen: {reason}",
        it: "L’IA ha lasciato questo a te: {reason}",
        ja: "AI があなたに任せました: {reason}", zh: "AI 把这项留给了你：{reason}",
        ko: "AI가 이 항목을 맡겼어요: {reason}", ru: "ИИ оставил это вам: {reason}" }
    /// When and where a conversation's candidate was received, on one line.
    received_inline(date: &str, location: &str) { en: "Received on {date} · {location}",
        pt: "Recebido em {date} · {location}", es: "Recibido el {date} · {location}",
        fr: "Reçu le {date} · {location}", de: "Empfangen am {date} · {location}",
        it: "Ricevuto il {date} · {location}", ja: "{date} に受信 · {location}",
        zh: "接收于 {date} · {location}", ko: "{date}에 받음 · {location}",
        ru: "Получено {date} · {location}" }
    /// When and where the candidate was received, on two lines.
    received_lines(date: &str, location: &str) { en: "Received on {date}\n{location}",
        pt: "Recebido em {date}\n{location}", es: "Recibido el {date}\n{location}",
        fr: "Reçu le {date}\n{location}", de: "Empfangen am {date}\n{location}",
        it: "Ricevuto il {date}\n{location}", ja: "{date} に受信\n{location}",
        zh: "接收于 {date}\n{location}", ko: "{date}에 받음\n{location}",
        ru: "Получено {date}\n{location}" }
    /// Capture counts in the loaded-rows footer.
    loaded_visible(loaded: usize, visible: usize) {
        en: "{loaded} loaded · {visible} visible",
        pt: "{loaded} carregados · {visible} visíveis",
        es: "{loaded} cargados · {visible} visibles",
        fr: "{loaded} chargés · {visible} visibles",
        de: "{loaded} geladen · {visible} sichtbar",
        it: "{loaded} caricati · {visible} visibili",
        ja: "{loaded} 件読み込み済み · {visible} 件表示",
        zh: "已加载 {loaded} · 可见 {visible}", ko: "{loaded}개 불러옴 · {visible}개 표시",
        ru: "загружено: {loaded} · видно: {visible}" }
    /// Line under the empty review queue: what the latest captures did.
    capture_activity(summary: &str) { en: "Latest captures: {summary}",
        pt: "Últimas capturas: {summary}", es: "Últimas capturas: {summary}",
        fr: "Dernières captures : {summary}", de: "Letzte Erfassungen: {summary}",
        it: "Ultime acquisizioni: {summary}", ja: "最近のキャプチャ: {summary}",
        zh: "最近的捕获：{summary}", ko: "최근 캡처: {summary}",
        ru: "Последние захваты: {summary}" }
}

plurals! {
    /// New candidates since the previous visit.
    briefing_new_candidates(count) {
        en: ["{count} new candidate", "{count} new candidates"],
        pt: ["{count} candidato novo", "{count} candidatos novos"],
        es: ["{count} candidato nuevo", "{count} candidatos nuevos"],
        fr: ["{count} nouveau candidat", "{count} nouveaux candidats"],
        de: ["{count} neuer Kandidat", "{count} neue Kandidaten"],
        it: ["{count} nuovo candidato", "{count} nuovi candidati"],
        ja: "新しい候補 {count} 件", zh: "{count} 个新候选", ko: "새 후보 {count}개",
        ru: ["{count} новый кандидат", "{count} новых кандидата", "{count} новых кандидатов"] }
    /// Decisions since the previous visit.
    briefing_decisions(count) {
        en: ["{count} decision", "{count} decisions"],
        pt: ["{count} decisão", "{count} decisões"],
        es: ["{count} decisión", "{count} decisiones"],
        fr: ["{count} décision", "{count} décisions"],
        de: ["{count} Entscheidung", "{count} Entscheidungen"],
        it: ["{count} decisione", "{count} decisioni"],
        ja: "決定 {count} 件", zh: "{count} 项决策", ko: "결정 {count}개",
        ru: ["{count} решение", "{count} решения", "{count} решений"] }
    /// Deliveries of context to the agent since the previous visit.
    briefing_deliveries_count(count) {
        en: ["{count} delivery to the agent", "{count} deliveries to the agent"],
        pt: ["{count} entrega ao agente", "{count} entregas ao agente"],
        es: ["{count} entrega al agente", "{count} entregas al agente"],
        fr: ["{count} livraison à l’agent", "{count} livraisons à l’agent"],
        de: ["{count} Lieferung an den Agenten", "{count} Lieferungen an den Agenten"],
        it: ["{count} consegna all’agente", "{count} consegne all’agente"],
        ja: "エージェントへの配信 {count} 件", zh: "向智能体交付 {count} 次",
        ko: "에이전트에 전달 {count}회",
        ru: ["{count} передача агенту", "{count} передачи агенту", "{count} передач агенту"] }
    /// Sessions behind the deliveries of the briefing.
    briefing_sessions_count(count) {
        en: ["{count} session", "{count} sessions"],
        pt: ["{count} sessão", "{count} sessões"],
        es: ["{count} sesión", "{count} sesiones"],
        fr: ["{count} session", "{count} sessions"],
        de: ["{count} Sitzung", "{count} Sitzungen"],
        it: ["{count} sessione", "{count} sessioni"],
        ja: "セッション {count} 件", zh: "{count} 个会话", ko: "세션 {count}개",
        ru: ["{count} сессия", "{count} сессии", "{count} сессий"] }
    /// How many sources a candidate's evidence has.
    sources_count(count) {
        en: ["{count} source", "{count} sources"],
        pt: ["{count} fonte", "{count} fontes"],
        es: ["{count} fuente", "{count} fuentes"],
        fr: ["{count} source", "{count} sources"],
        de: ["{count} Quelle", "{count} Quellen"],
        it: ["{count} fonte", "{count} fonti"],
        ja: "ソース {count} 件", zh: "{count} 个来源", ko: "출처 {count}개",
        ru: ["{count} источник", "{count} источника", "{count} источников"] }
    /// Proposals still waiting for review.
    capture_proposals(count) {
        en: ["{count} proposal", "{count} proposals"], pt: ["{count} proposta", "{count} propostas"],
        es: ["{count} propuesta", "{count} propuestas"], fr: ["{count} proposition", "{count} propositions"],
        de: ["{count} Vorschlag", "{count} Vorschläge"], it: ["{count} proposta", "{count} proposte"],
        ja: "提案 {count} 件", zh: "{count} 条提案", ko: "제안 {count}개",
        ru: ["{count} предложение", "{count} предложения", "{count} предложений"] }
    /// Proposals confirmed.
    capture_confirmed(count) {
        en: ["{count} confirmed", "{count} confirmed"], pt: ["{count} confirmada", "{count} confirmadas"],
        es: ["{count} confirmada", "{count} confirmadas"], fr: ["{count} confirmée", "{count} confirmées"],
        de: ["{count} bestätigt", "{count} bestätigt"], it: ["{count} confermata", "{count} confermate"],
        ja: "確定 {count} 件", zh: "已确认 {count} 条", ko: "확인 {count}개",
        ru: ["подтверждено: {count}", "подтверждено: {count}", "подтверждено: {count}"] }
    /// Proposals rejected.
    capture_rejected(count) {
        en: ["{count} rejected", "{count} rejected"], pt: ["{count} rejeitada", "{count} rejeitadas"],
        es: ["{count} rechazada", "{count} rechazadas"], fr: ["{count} rejetée", "{count} rejetées"],
        de: ["{count} abgelehnt", "{count} abgelehnt"], it: ["{count} rifiutata", "{count} rifiutate"],
        ja: "却下 {count} 件", zh: "已拒绝 {count} 条", ko: "거부 {count}개",
        ru: ["отклонено: {count}", "отклонено: {count}", "отклонено: {count}"] }
    /// Proposals postponed.
    capture_postponed(count) {
        en: ["{count} postponed", "{count} postponed"], pt: ["{count} adiada", "{count} adiadas"],
        es: ["{count} aplazada", "{count} aplazadas"], fr: ["{count} reportée", "{count} reportées"],
        de: ["{count} zurückgestellt", "{count} zurückgestellt"], it: ["{count} rimandata", "{count} rimandate"],
        ja: "保留 {count} 件", zh: "已推迟 {count} 条", ko: "미룸 {count}개",
        ru: ["отложено: {count}", "отложено: {count}", "отложено: {count}"] }
    /// Summary: captures waiting for analysis.
    capture_waiting(count) {
        en: ["{count} waiting for analysis", "{count} waiting for analysis"], pt: ["{count} aguardando análise", "{count} aguardando análise"],
        es: ["{count} esperando análisis", "{count} esperando análisis"], fr: ["{count} en attente d’analyse", "{count} en attente d’analyse"],
        de: ["{count} wartet auf Analyse", "{count} warten auf Analyse"], it: ["{count} in attesa di analisi", "{count} in attesa di analisi"],
        ja: "分析待ち {count} 件", zh: "{count} 条等待分析", ko: "분석 대기 {count}개",
        ru: ["ожидает анализа: {count}", "ожидает анализа: {count}", "ожидает анализа: {count}"] }
    /// Summary: captures analysed.
    capture_analysed(count) {
        en: ["{count} analysed", "{count} analysed"], pt: ["{count} analisada", "{count} analisadas"],
        es: ["{count} analizada", "{count} analizadas"], fr: ["{count} analysée", "{count} analysées"],
        de: ["{count} analysiert", "{count} analysiert"], it: ["{count} analizzata", "{count} analizzate"],
        ja: "分析済み {count} 件", zh: "已分析 {count} 条", ko: "분석됨 {count}개",
        ru: ["проанализировано: {count}", "проанализировано: {count}", "проанализировано: {count}"] }
    /// Summary: captures whose analysis failed.
    capture_failed_count(count) {
        en: ["{count} failed", "{count} failed"], pt: ["{count} falhou", "{count} falharam"],
        es: ["{count} falló", "{count} fallaron"], fr: ["{count} échec", "{count} échecs"],
        de: ["{count} fehlgeschlagen", "{count} fehlgeschlagen"], it: ["{count} non riuscita", "{count} non riuscite"],
        ja: "失敗 {count} 件", zh: "{count} 条失败", ko: "실패 {count}개",
        ru: ["ошибок: {count}", "ошибок: {count}", "ошибок: {count}"] }
    /// Attempt number, shown only after a retry.
    capture_attempt(count) {
        en: ["attempt {count}", "attempt {count}"], pt: ["tentativa {count}", "tentativa {count}"],
        es: ["intento {count}", "intento {count}"], fr: ["tentative {count}", "tentative {count}"],
        de: ["Versuch {count}", "Versuch {count}"], it: ["tentativo {count}", "tentativo {count}"],
        ja: "{count} 回目", zh: "第 {count} 次尝试", ko: "{count}번째 시도",
        ru: ["попытка {count}", "попытка {count}", "попытка {count}"] }
}

/// Deliveries to the agent with the sessions that made them.
pub fn briefing_deliveries(deliveries: usize, sessions: usize) -> String {
    format!(
        "{} ({})",
        briefing_deliveries_count(deliveries),
        briefing_sessions_count(sessions)
    )
}

// Conflicts the automatic review left for the person: two sides, three
// actions, each with the effect it has.
strings! {
    /// Region label of the conflict panel.
    conflict_title { en: "Conflict", pt: "Conflito", es: "Conflicto", fr: "Conflit",
        de: "Konflikt", it: "Conflitto", ja: "競合", zh: "冲突", ko: "충돌",
        ru: "Конфликт" }
    /// Accessible name of the conflict panel.
    conflict_aria { en: "Conflict between two items", pt: "Conflito entre dois itens",
        es: "Conflicto entre dos elementos", fr: "Conflit entre deux éléments",
        de: "Konflikt zwischen zwei Einträgen", it: "Conflitto tra due elementi",
        ja: "2 つの項目の競合", zh: "两项之间的冲突", ko: "두 항목 간의 충돌",
        ru: "Конфликт между двумя элементами" }
    /// Label of the side that is selected in the queue.
    conflict_side_this { en: "This one", pt: "Esta", es: "Esta", fr: "Celle-ci",
        de: "Diese", it: "Questa", ja: "こちら", zh: "这一条", ko: "이 항목",
        ru: "Эта" }
    /// Label of the side it contradicts.
    conflict_side_other { en: "The other", pt: "A outra", es: "La otra", fr: "L’autre",
        de: "Die andere", it: "L’altra", ja: "もう一方", zh: "另一条", ko: "다른 항목",
        ru: "Другая" }
    /// State of a side that still waits for review.
    conflict_state_candidate { en: "Waiting for review", pt: "Aguardando revisão",
        es: "Esperando revisión", fr: "En attente de revue", de: "Wartet auf Prüfung",
        it: "In attesa di revisione", ja: "レビュー待ち", zh: "等待审核",
        ko: "검토 대기 중", ru: "Ждёт проверки" }
    /// State of a side that is a decision in force.
    conflict_state_in_force { en: "In force", pt: "Em vigor", es: "En vigor",
        fr: "En vigueur", de: "Gültig", it: "In vigore", ja: "有効", zh: "生效中",
        ko: "적용 중", ru: "Действует" }
    /// Origin of a side that came from a captured session, not from a file.
    conflict_origin_capture { en: "From a captured session",
        pt: "De uma sessão capturada", es: "De una sesión capturada",
        fr: "D’une session capturée", de: "Aus einer erfassten Sitzung",
        it: "Da una sessione catturata", ja: "キャプチャしたセッション由来",
        zh: "来自已捕获的会话", ko: "캡처한 세션에서", ru: "Из захваченного сеанса" }
    /// Action: keep the selected side.
    conflict_keep_this { en: "Keep this one", pt: "Ficar com esta", es: "Quedarme con esta",
        fr: "Garder celle-ci", de: "Diese behalten", it: "Tenere questa",
        ja: "こちらを残す", zh: "保留这一条", ko: "이 항목 유지", ru: "Оставить эту" }
    /// Action: keep the other side.
    conflict_keep_other { en: "Keep the other", pt: "Ficar com a outra",
        es: "Quedarme con la otra", fr: "Garder l’autre", de: "Die andere behalten",
        it: "Tenere l’altra", ja: "もう一方を残す", zh: "保留另一条", ko: "다른 항목 유지",
        ru: "Оставить другую" }
    /// Action: keep both, each with a scope.
    conflict_keep_both { en: "Both apply", pt: "As duas valem", es: "Valen las dos",
        fr: "Les deux valent", de: "Beide gelten", it: "Valgono entrambe",
        ja: "両方有効", zh: "两条都有效", ko: "둘 다 유효", ru: "Действуют обе" }
    /// Effect of keeping this one when the other is a decision in force.
    conflict_effect_replaces { en: "This one replaces the decision in force, which stays in the history.",
        pt: "Esta passa a valer e substitui a decisão em vigor, que fica no histórico.",
        es: "Esta pasa a valer y sustituye la decisión en vigor, que queda en el historial.",
        fr: "Celle-ci remplace la décision en vigueur, qui reste dans l’historique.",
        de: "Diese ersetzt die gültige Entscheidung, die im Verlauf bleibt.",
        it: "Questa sostituisce la decisione in vigore, che resta nella cronologia.",
        ja: "こちらが有効な決定に置き換わり、元の決定は履歴に残ります。",
        zh: "这一条取代生效中的决策，旧决策保留在历史中。",
        ko: "이 항목이 적용 중인 결정을 대체하고, 기존 결정은 기록에 남아요.",
        ru: "Эта заменяет действующее решение, оно остаётся в истории." }
    /// Effect of keeping this one when the other is a candidate.
    conflict_effect_rejects_other { en: "This one is accepted and the other is rejected.",
        pt: "Esta é aceita e a outra é rejeitada.",
        es: "Esta se acepta y la otra se rechaza.",
        fr: "Celle-ci est acceptée et l’autre est rejetée.",
        de: "Diese wird angenommen, die andere abgelehnt.",
        it: "Questa viene accettata e l’altra rifiutata.",
        ja: "こちらを承認し、もう一方は却下します。",
        zh: "接受这一条，拒绝另一条。",
        ko: "이 항목은 수락하고 다른 항목은 거절해요.",
        ru: "Эта принимается, другая отклоняется." }
    /// Why keeping this one is not offered: a rule cannot replace a decision.
    conflict_effect_cannot_replace { en: "A rule cannot replace a decision in force: pick another option.",
        pt: "Uma regra não substitui uma decisão em vigor: escolha outra opção.",
        es: "Una regla no sustituye una decisión en vigor: elige otra opción.",
        fr: "Une règle ne remplace pas une décision en vigueur : choisissez une autre option.",
        de: "Eine Regel ersetzt keine gültige Entscheidung: wähle eine andere Option.",
        it: "Una regola non sostituisce una decisione in vigore: scegli un’altra opzione.",
        ja: "ルールは有効な決定を置き換えられません。別の選択肢を選んでください。",
        zh: "规则不能取代生效中的决策，请选择其他选项。",
        ko: "규칙은 적용 중인 결정을 대체할 수 없어요. 다른 선택지를 고르세요.",
        ru: "Правило не заменяет действующее решение: выберите другой вариант." }
    /// Effect of keeping the other when it is in force.
    conflict_effect_other_stays { en: "The one in force stays and this one is rejected.",
        pt: "A que está em vigor continua e esta é rejeitada.",
        es: "La que está en vigor sigue y esta se rechaza.",
        fr: "Celle en vigueur reste et celle-ci est rejetée.",
        de: "Die gültige bleibt, diese wird abgelehnt.",
        it: "Quella in vigore resta e questa viene rifiutata.",
        ja: "有効なものはそのまま残り、こちらは却下します。",
        zh: "生效中的保持不变，拒绝这一条。",
        ko: "적용 중인 항목은 그대로 두고 이 항목은 거절해요.",
        ru: "Действующее остаётся, эта отклоняется." }
    /// Effect of keeping the other when it is a candidate.
    conflict_effect_other_accepted { en: "The other is accepted and this one is rejected.",
        pt: "A outra é aceita e esta é rejeitada.",
        es: "La otra se acepta y esta se rechaza.",
        fr: "L’autre est acceptée et celle-ci est rejetée.",
        de: "Die andere wird angenommen, diese abgelehnt.",
        it: "L’altra viene accettata e questa rifiutata.",
        ja: "もう一方を承認し、こちらは却下します。",
        zh: "接受另一条，拒绝这一条。",
        ko: "다른 항목은 수락하고 이 항목은 거절해요.",
        ru: "Другая принимается, эта отклоняется." }
    /// Effect of keeping both.
    conflict_effect_both { en: "Both stay, each with the scope you write.",
        pt: "As duas ficam, cada uma com o escopo que você escrever.",
        es: "Las dos se quedan, cada una con el alcance que escribas.",
        fr: "Les deux restent, chacune avec la portée que vous écrivez.",
        de: "Beide bleiben, jede mit dem Geltungsbereich, den du schreibst.",
        it: "Restano entrambe, ciascuna con l’ambito che scrivi.",
        ja: "両方を残し、それぞれに書いた適用範囲を付けます。",
        zh: "两条都保留，各自带上你写的适用范围。",
        ko: "둘 다 남기고, 각각 적은 적용 범위를 붙여요.",
        ru: "Остаются обе, каждая с областью применения, которую вы напишете." }
    /// Title of the step that asks for the two scopes.
    conflict_scope_title { en: "Where does each one apply?", pt: "Onde cada uma vale?",
        es: "¿Dónde vale cada una?", fr: "Où chacune s’applique-t-elle ?",
        de: "Wo gilt jede?", it: "Dove vale ciascuna?",
        ja: "それぞれどこに適用されますか。", zh: "各自适用于哪里？",
        ko: "각각 어디에 적용되나요?", ru: "Где действует каждая?" }
    /// Field: scope of the selected side.
    conflict_scope_this { en: "Scope of this one", pt: "Escopo desta",
        es: "Alcance de esta", fr: "Portée de celle-ci", de: "Geltungsbereich dieser",
        it: "Ambito di questa", ja: "こちらの適用範囲", zh: "这一条的适用范围",
        ko: "이 항목의 적용 범위", ru: "Область применения этой" }
    /// Field: scope of the other side.
    conflict_scope_other { en: "Scope of the other", pt: "Escopo da outra",
        es: "Alcance de la otra", fr: "Portée de l’autre", de: "Geltungsbereich der anderen",
        it: "Ambito dell’altra", ja: "もう一方の適用範囲", zh: "另一条的适用范围",
        ko: "다른 항목의 적용 범위", ru: "Область применения другой" }
    /// Hint and placeholder of a scope field.
    conflict_scope_hint { en: "One line, for example: only in the public API.",
        pt: "Uma linha, por exemplo: só na API pública.",
        es: "Una línea, por ejemplo: solo en la API pública.",
        fr: "Une ligne, par exemple : seulement dans l’API publique.",
        de: "Eine Zeile, zum Beispiel: nur in der öffentlichen API.",
        it: "Una riga, per esempio: solo nell’API pubblica.",
        ja: "1 行で入力します。例: 公開 API のみ。",
        zh: "一行即可，例如：仅限公共 API。",
        ko: "한 줄로 적어요. 예: 공개 API에서만.",
        ru: "Одна строка, например: только в публичном API." }
    /// Confirms both with their scopes.
    conflict_scope_confirm { en: "Keep both", pt: "Manter as duas", es: "Mantener las dos",
        fr: "Garder les deux", de: "Beide behalten", it: "Mantieni entrambe",
        ja: "両方を残す", zh: "保留两条", ko: "둘 다 유지", ru: "Оставить обе" }
    /// Goes back from the scope step.
    conflict_scope_back { en: "Back", pt: "Voltar", es: "Volver", fr: "Retour",
        de: "Zurück", it: "Indietro", ja: "戻る", zh: "返回", ko: "뒤로", ru: "Назад" }
    /// Notice after keeping this one.
    notice_conflict_this { en: "Kept this one.", pt: "Ficou com esta.",
        es: "Te quedaste con esta.", fr: "Celle-ci est gardée.", de: "Diese bleibt.",
        it: "Hai tenuto questa.", ja: "こちらを残しました。", zh: "已保留这一条。",
        ko: "이 항목을 유지했어요.", ru: "Эта оставлена." }
    /// Notice after keeping the other.
    notice_conflict_other { en: "Kept the other.", pt: "Ficou com a outra.",
        es: "Te quedaste con la otra.", fr: "L’autre est gardée.", de: "Die andere bleibt.",
        it: "Hai tenuto l’altra.", ja: "もう一方を残しました。", zh: "已保留另一条。",
        ko: "다른 항목을 유지했어요.", ru: "Другая оставлена." }
    /// Notice after keeping both.
    notice_conflict_both { en: "Both kept, each with its scope.",
        pt: "As duas ficaram, cada uma com o escopo.",
        es: "Las dos se quedaron, cada una con su alcance.",
        fr: "Les deux sont gardées, chacune avec sa portée.",
        de: "Beide bleiben, jede mit ihrem Geltungsbereich.",
        it: "Restano entrambe, ciascuna con il suo ambito.",
        ja: "両方を適用範囲付きで残しました。", zh: "两条都已保留，各带适用范围。",
        ko: "둘 다 적용 범위와 함께 유지했어요.", ru: "Обе оставлены, каждая со своей областью." }
    /// Error: the choice could not be applied, usually because a side changed.
    conflict_failed { en: "Could not apply the choice: the conflict changed. Refresh the queue.",
        pt: "Não foi possível aplicar a escolha: o conflito mudou. Atualize a fila.",
        es: "No se pudo aplicar la elección: el conflicto cambió. Actualiza la cola.",
        fr: "Impossible d’appliquer le choix : le conflit a changé. Actualisez la file.",
        de: "Die Auswahl konnte nicht angewendet werden: Der Konflikt hat sich geändert. Aktualisiere die Warteschlange.",
        it: "Impossibile applicare la scelta: il conflitto è cambiato. Aggiorna la coda.",
        ja: "選択を適用できませんでした。競合が変わっています。キューを更新してください。",
        zh: "无法应用所选操作：冲突已变化。请刷新队列。",
        ko: "선택을 적용하지 못했어요. 충돌이 바뀌었어요. 대기열을 새로고침하세요.",
        ru: "Не удалось применить выбор: конфликт изменился. Обновите очередь." }
}

formats! {
    /// Origin of a side that came from a file.
    conflict_origin_file(path: &str) { en: "From {path}", pt: "De {path}", es: "De {path}",
        fr: "De {path}", de: "Aus {path}", it: "Da {path}", ja: "{path} から",
        zh: "来自 {path}", ko: "{path}에서", ru: "Из {path}" }
    /// When a side that still waits was proposed.
    conflict_date_candidate(date: &str) { en: "Proposed on {date}",
        pt: "Proposta em {date}", es: "Propuesta el {date}", fr: "Proposée le {date}",
        de: "Vorgeschlagen am {date}", it: "Proposta il {date}", ja: "{date} に提案",
        zh: "提出于 {date}", ko: "{date}에 제안됨", ru: "Предложено {date}" }
    /// Since when a side has been in force.
    conflict_date_in_force(date: &str) { en: "In force since {date}",
        pt: "Em vigor desde {date}", es: "En vigor desde el {date}",
        fr: "En vigueur depuis le {date}", de: "Gültig seit {date}",
        it: "In vigore dal {date}", ja: "{date} から有効", zh: "自 {date} 起生效",
        ko: "{date}부터 적용 중", ru: "Действует с {date}" }
    /// The scope already recorded on a side.
    conflict_scope_line(scope: &str) { en: "Scope: {scope}", pt: "Escopo: {scope}",
        es: "Alcance: {scope}", fr: "Portée : {scope}", de: "Geltungsbereich: {scope}",
        it: "Ambito: {scope}", ja: "適用範囲: {scope}", zh: "适用范围：{scope}",
        ko: "적용 범위: {scope}", ru: "Область применения: {scope}" }
}
