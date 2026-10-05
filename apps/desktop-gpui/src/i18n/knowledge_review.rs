//! Copy for the knowledge_review area. See [`crate::i18n`] for how entries are declared.

use super::{formats, strings};

strings! {
    /// Page title.
    title { en: "Review knowledge", pt: "Revisar conhecimento", es: "Revisar conocimiento",
        fr: "Revoir les connaissances", de: "Wissen prüfen", it: "Rivedi la conoscenza",
        ja: "知識をレビュー", zh: "审阅知识", ko: "지식 검토", ru: "Проверка знаний" }
    /// Disclaimer under the title.
    advisory { en: "Advisory review, without changing knowledge. A tension is not a proven conflict; citing does not guarantee a correct inference.",
        pt: "Revisão consultiva, sem alterar conhecimento. Tensão não é conflito comprovado; citar não garante uma inferência correta.",
        es: "Revisión consultiva, sin modificar el conocimiento. Una tensión no es un conflicto comprobado; citar no garantiza una inferencia correcta.",
        fr: "Revue consultative, sans modifier les connaissances. Une tension n’est pas un conflit avéré ; citer ne garantit pas une inférence correcte.",
        de: "Beratende Prüfung, ohne das Wissen zu verändern. Eine Spannung ist kein nachgewiesener Konflikt; ein Zitat garantiert keine korrekte Schlussfolgerung.",
        it: "Revisione consultiva, senza modificare la conoscenza. Una tensione non è un conflitto provato; citare non garantisce un’inferenza corretta.",
        ja: "助言としてのレビューで、知識は変更しません。緊張関係は確認済みの矛盾ではなく、引用しても推論が正しいとは限りません。",
        zh: "仅供参考的审阅，不会修改知识。张力不等于已证实的冲突；有引用也不能保证推断正确。",
        ko: "참고용 검토이며 지식은 바꾸지 않아요. 긴장은 확인된 충돌이 아니며, 인용이 올바른 추론을 보장하지도 않아요.",
        ru: "Консультативная проверка, без изменения знаний. Напряжение — не доказанный конфликт; цитата не гарантирует верный вывод." }
    /// Label (icon caption) of the empty states.
    panel_label { en: "Review", pt: "Revisão", es: "Revisión", fr: "Revue", de: "Prüfung",
        it: "Revisione", ja: "レビュー", zh: "审阅", ko: "검토", ru: "Проверка" }
    /// Title when the review cannot run.
    unavailable_title { en: "Review unavailable", pt: "Revisão indisponível",
        es: "Revisión no disponible", fr: "Revue indisponible",
        de: "Prüfung nicht verfügbar", it: "Revisione non disponibile",
        ja: "レビューは利用できません", zh: "审阅不可用", ko: "검토를 사용할 수 없어요",
        ru: "Проверка недоступна" }
    /// Body when the review cannot run.
    unavailable_body { en: "Select a project with storage available.",
        pt: "Selecione um projeto com armazenamento disponível.",
        es: "Selecciona un proyecto con almacenamiento disponible.",
        fr: "Sélectionnez un projet avec un stockage disponible.",
        de: "Wähle ein Projekt mit verfügbarem Speicher.",
        it: "Seleziona un progetto con archiviazione disponibile.",
        ja: "ストレージが利用できるプロジェクトを選んでください。",
        zh: "请选择一个存储可用的项目。",
        ko: "저장소를 사용할 수 있는 프로젝트를 선택해 주세요.",
        ru: "Выберите проект с доступным хранилищем." }
    /// Status while a cancelled call is still returning.
    cancelling { en: "Cancelling… waiting for the call in progress to return.",
        pt: "Cancelando… aguardando a chamada em curso retornar.",
        es: "Cancelando… esperando a que termine la llamada en curso.",
        fr: "Annulation… en attente du retour de l’appel en cours.",
        de: "Wird abgebrochen… Warte, bis der laufende Aufruf zurückkehrt.",
        it: "Annullamento… in attesa che la chiamata in corso termini.",
        ja: "キャンセル中… 実行中の呼び出しが戻るのを待っています。",
        zh: "正在取消… 等待进行中的调用返回。",
        ko: "취소하는 중… 진행 중인 호출이 끝나기를 기다리고 있어요.",
        ru: "Отмена… ждём завершения текущего вызова." }
    /// Status while the review runs.
    checking { en: "Checking knowledge…", pt: "Verificando conhecimento…",
        es: "Verificando el conocimiento…", fr: "Vérification des connaissances…",
        de: "Wissen wird geprüft…", it: "Verifica della conoscenza…",
        ja: "知識を確認中…", zh: "正在检查知识…", ko: "지식을 확인하는 중…",
        ru: "Проверка знаний…" }
    /// Title of the empty state before any check.
    idle_title { en: "Check the recorded knowledge", pt: "Verifique o conhecimento registrado",
        es: "Verifica el conocimiento registrado", fr: "Vérifiez les connaissances enregistrées",
        de: "Prüfe das erfasste Wissen", it: "Verifica la conoscenza registrata",
        ja: "記録された知識を確認", zh: "检查已记录的知识", ko: "기록된 지식 확인하기",
        ru: "Проверьте записанные знания" }
    /// Body of the empty state before any check.
    idle_body { en: "Checking locally sends no text. AI requires explicit confirmation.",
        pt: "Verificar localmente não envia textos. A IA exige confirmação explícita.",
        es: "Verificar localmente no envía textos. La IA requiere confirmación explícita.",
        fr: "La vérification locale n’envoie aucun texte. L’IA exige une confirmation explicite.",
        de: "Die lokale Prüfung sendet keine Texte. Die KI erfordert eine ausdrückliche Bestätigung.",
        it: "La verifica locale non invia testi. L’IA richiede una conferma esplicita.",
        ja: "ローカルでの確認はテキストを送信しません。AIの利用には明示的な確認が必要です。",
        zh: "本地检查不会发送文本。使用 AI 需要明确确认。",
        ko: "로컬 확인은 텍스트를 보내지 않아요. AI는 명시적인 확인이 필요해요.",
        ru: "Локальная проверка ничего не отправляет. ИИ требует явного подтверждения." }
    /// Error banner when the knowledge could not be read.
    read_failed { en: "Couldn’t read the knowledge. Try checking locally again.",
        pt: "Não foi possível ler o conhecimento. Tente verificar localmente novamente.",
        es: "No se pudo leer el conocimiento. Intenta verificar localmente de nuevo.",
        fr: "Impossible de lire les connaissances. Essayez de vérifier à nouveau en local.",
        de: "Das Wissen konnte nicht gelesen werden. Versuche die lokale Prüfung erneut.",
        it: "Impossibile leggere la conoscenza. Riprova la verifica locale.",
        ja: "知識を読み込めませんでした。もう一度ローカルで確認してください。",
        zh: "无法读取知识。请再次尝试本地检查。",
        ko: "지식을 읽을 수 없어요. 로컬 확인을 다시 시도해 주세요.",
        ru: "Не удалось прочитать знания. Попробуйте ещё раз выполнить локальную проверку." }
    /// Section: the local checks.
    local_checks { en: "Local checks", pt: "Verificações locais", es: "Verificaciones locales",
        fr: "Vérifications locales", de: "Lokale Prüfungen", it: "Verifiche locali",
        ja: "ローカル確認", zh: "本地检查", ko: "로컬 확인", ru: "Локальные проверки" }
    /// The local checks finished.
    local_done { en: "Local checks complete.", pt: "Verificações locais concluídas.",
        es: "Verificaciones locales completadas.", fr: "Vérifications locales terminées.",
        de: "Lokale Prüfungen abgeschlossen.", it: "Verifiche locali completate.",
        ja: "ローカル確認が完了しました。", zh: "本地检查已完成。",
        ko: "로컬 확인을 마쳤어요.", ru: "Локальные проверки завершены." }
    /// The local checks did not finish.
    local_incomplete { en: "Local checks incomplete.", pt: "Verificações locais incompletas.",
        es: "Verificaciones locales incompletas.", fr: "Vérifications locales incomplètes.",
        de: "Lokale Prüfungen unvollständig.", it: "Verifiche locali incomplete.",
        ja: "ローカル確認は未完了です。", zh: "本地检查未完成。",
        ko: "로컬 확인이 끝나지 않았어요.", ru: "Локальные проверки не завершены." }
    /// Section: the AI review.
    ai_review { en: "AI review", pt: "Revisão IA", es: "Revisión con IA", fr: "Revue par l’IA",
        de: "KI-Prüfung", it: "Revisione con IA", ja: "AIレビュー", zh: "AI 审阅",
        ko: "AI 검토", ru: "Проверка ИИ" }
    /// AI review status: not requested.
    status_not_requested { en: "Not requested; no text sent.",
        pt: "Não solicitada; nenhum texto enviado.",
        es: "No solicitada; no se envió ningún texto.",
        fr: "Non demandée ; aucun texte envoyé.",
        de: "Nicht angefordert; kein Text gesendet.",
        it: "Non richiesta; nessun testo inviato.",
        ja: "未実行です。テキストは送信されていません。",
        zh: "未请求；未发送任何文本。",
        ko: "요청하지 않았어요. 보낸 텍스트가 없어요.",
        ru: "Не запрошена; текст не отправлялся." }
    /// AI review status: completed.
    status_completed { en: "Completed within the selected coverage.",
        pt: "Concluída dentro da cobertura selecionada.",
        es: "Completada dentro de la cobertura seleccionada.",
        fr: "Terminée dans la couverture sélectionnée.",
        de: "Abgeschlossen im gewählten Umfang.",
        it: "Completata entro la copertura selezionata.",
        ja: "選択した対象範囲で完了しました。",
        zh: "已在所选覆盖范围内完成。",
        ko: "선택한 범위 안에서 완료했어요.",
        ru: "Завершена в пределах выбранного охвата." }
    /// AI review status: partial.
    status_partial { en: "Partial; not all knowledge was reviewed.",
        pt: "Parcial; nem todo o conhecimento foi revisado.",
        es: "Parcial; no se revisó todo el conocimiento.",
        fr: "Partielle ; toutes les connaissances n’ont pas été revues.",
        de: "Teilweise; nicht das gesamte Wissen wurde geprüft.",
        it: "Parziale; non tutta la conoscenza è stata rivista.",
        ja: "一部のみです。すべての知識をレビューしたわけではありません。",
        zh: "部分完成；并非所有知识都已审阅。",
        ko: "일부만 했어요. 모든 지식을 검토하지는 않았어요.",
        ru: "Частично; проверены не все знания." }
    /// AI review status: unavailable.
    status_unavailable { en: "Unavailable; check the provider and consent in Settings.",
        pt: "Indisponível; confira provedor e consentimento em Configurações.",
        es: "No disponible; revisa el proveedor y el consentimiento en Ajustes.",
        fr: "Indisponible ; vérifiez le fournisseur et le consentement dans Réglages.",
        de: "Nicht verfügbar; prüfe Anbieter und Einwilligung in den Einstellungen.",
        it: "Non disponibile; controlla provider e consenso nelle Impostazioni.",
        ja: "利用できません。設定でプロバイダーと同意を確認してください。",
        zh: "不可用；请在设置中检查提供商和同意状态。",
        ko: "사용할 수 없어요. 설정에서 공급자와 동의를 확인해 주세요.",
        ru: "Недоступна; проверьте провайдера и согласие в настройках." }
    /// AI review status: failed.
    status_failed { en: "Failed or invalid response; local findings preserved.",
        pt: "Falha ou resposta inválida; achados locais preservados.",
        es: "Falló o respuesta no válida; se conservan los hallazgos locales.",
        fr: "Échec ou réponse invalide ; constats locaux conservés.",
        de: "Fehlgeschlagen oder ungültige Antwort; lokale Befunde bleiben erhalten.",
        it: "Non riuscita o risposta non valida; riscontri locali conservati.",
        ja: "失敗したか、無効な応答でした。ローカルの検出結果は保持されています。",
        zh: "失败或响应无效；本地发现已保留。",
        ko: "실패했거나 응답이 올바르지 않아요. 로컬 결과는 그대로 남아 있어요.",
        ru: "Сбой или недопустимый ответ; локальные находки сохранены." }
    /// AI review status: cancelled.
    status_cancelled { en: "Cancelled; local findings preserved.",
        pt: "Cancelada; achados locais preservados.",
        es: "Cancelada; se conservan los hallazgos locales.",
        fr: "Annulée ; constats locaux conservés.",
        de: "Abgebrochen; lokale Befunde bleiben erhalten.",
        it: "Annullata; riscontri locali conservati.",
        ja: "キャンセルしました。ローカルの検出結果は保持されています。",
        zh: "已取消；本地发现已保留。",
        ko: "취소했어요. 로컬 결과는 그대로 남아 있어요.",
        ru: "Отменена; локальные находки сохранены." }
    /// Section: findings from the local checks.
    local_findings { en: "Local findings", pt: "Achados locais", es: "Hallazgos locales",
        fr: "Constats locaux", de: "Lokale Befunde", it: "Riscontri locali",
        ja: "ローカルの検出結果", zh: "本地发现", ko: "로컬 결과", ru: "Локальные находки" }
    /// Section: findings from the AI review.
    hypotheses { en: "Advisory hypotheses", pt: "Hipóteses consultivas",
        es: "Hipótesis consultivas", fr: "Hypothèses consultatives",
        de: "Beratende Hypothesen", it: "Ipotesi consultive", ja: "助言的な仮説",
        zh: "参考性假设", ko: "참고용 가설", ru: "Консультативные гипотезы" }
    /// A section with no findings.
    no_findings { en: "No findings in this coverage. That does not prove the absence of problems.",
        pt: "Sem achados nesta cobertura. Isso não comprova ausência de problemas.",
        es: "Sin hallazgos en esta cobertura. Eso no demuestra la ausencia de problemas.",
        fr: "Aucun constat dans cette couverture. Cela ne prouve pas l’absence de problèmes.",
        de: "Keine Befunde in diesem Umfang. Das beweist nicht, dass es keine Probleme gibt.",
        it: "Nessun riscontro in questa copertura. Ciò non dimostra l’assenza di problemi.",
        ja: "この対象範囲では検出結果はありません。問題がないことの証明にはなりません。",
        zh: "此覆盖范围内没有发现。这并不能证明不存在问题。",
        ko: "이 범위에서는 결과가 없어요. 문제가 없다는 뜻은 아니에요.",
        ru: "В этом охвате находок нет. Это не доказывает отсутствие проблем." }
    /// Aria label of the button that opens a cited decision.
    open_decision_aria { en: "Open cited decision", pt: "Abrir decisão citada",
        es: "Abrir decisión citada", fr: "Ouvrir la décision citée",
        de: "Zitierte Entscheidung öffnen", it: "Apri la decisione citata",
        ja: "引用された決定を開く", zh: "打开被引用的决策", ko: "인용된 결정 열기",
        ru: "Открыть процитированное решение" }
    /// Button that opens a cited decision.
    open_decision { en: "Open decision", pt: "Abrir decisão", es: "Abrir decisión",
        fr: "Ouvrir la décision", de: "Entscheidung öffnen", it: "Apri decisione",
        ja: "決定を開く", zh: "打开决策", ko: "결정 열기", ru: "Открыть решение" }
    /// Section: coverage of the review.
    coverage { en: "Coverage", pt: "Cobertura", es: "Cobertura", fr: "Couverture",
        de: "Abdeckung", it: "Copertura", ja: "対象範囲", zh: "覆盖范围", ko: "범위",
        ru: "Охват" }
    /// Question before sending texts to the provider.
    confirm_prompt { en: "Send the selected texts of decisions, rules, proposals and links to the configured provider? It sends no code and not the folder. It does not replace the consent currently in effect in Settings.",
        pt: "Enviar ao provedor configurado textos de decisões, regras, propostas e vínculos selecionados? Não envia código nem a pasta. Não substitui o consentimento vigente em Configurações.",
        es: "¿Enviar al proveedor configurado los textos seleccionados de decisiones, reglas, propuestas y vínculos? No envía código ni la carpeta. No sustituye el consentimiento vigente en Ajustes.",
        fr: "Envoyer au fournisseur configuré les textes sélectionnés des décisions, règles, propositions et liens ? Aucun code ni le dossier n’est envoyé. Cela ne remplace pas le consentement en vigueur dans Réglages.",
        de: "Ausgewählte Texte von Entscheidungen, Regeln, Vorschlägen und Verknüpfungen an den konfigurierten Anbieter senden? Es werden weder Code noch der Ordner gesendet. Ersetzt nicht die geltende Einwilligung in den Einstellungen.",
        it: "Inviare al provider configurato i testi selezionati di decisioni, regole, proposte e collegamenti? Non invia codice né la cartella. Non sostituisce il consenso in vigore nelle Impostazioni.",
        ja: "選択した決定・ルール・提案・リンクのテキストを、設定済みのプロバイダーに送信しますか？コードやフォルダーは送信しません。設定で有効な同意の代わりにはなりません。",
        zh: "要把所选的决策、规则、提案和关联的文本发送给已配置的提供商吗？不会发送代码或文件夹。这不能替代设置中当前生效的同意。",
        ko: "선택한 결정, 규칙, 제안, 연결의 텍스트를 설정된 공급자에게 보낼까요? 코드와 폴더는 보내지 않아요. 설정에 있는 현재 동의를 대신하지는 않아요.",
        ru: "Отправить настроенному провайдеру выбранные тексты решений, правил, предложений и ссылок? Код и папка не отправляются. Это не заменяет действующее согласие в настройках." }
    /// Button (and aria label): run the local checks.
    check_locally { en: "Check locally", pt: "Verificar localmente",
        es: "Verificar localmente", fr: "Vérifier en local", de: "Lokal prüfen",
        it: "Verifica in locale", ja: "ローカルで確認", zh: "本地检查", ko: "로컬 확인",
        ru: "Проверить локально" }
    /// Aria label of the AI review button.
    review_ai_aria { en: "Review with AI after confirmation", pt: "Revisar com IA após confirmação",
        es: "Revisar con IA tras confirmar", fr: "Revoir avec l’IA après confirmation",
        de: "Nach Bestätigung mit KI prüfen", it: "Rivedi con IA dopo la conferma",
        ja: "確認後にAIでレビュー", zh: "确认后用 AI 审阅", ko: "확인 후 AI로 검토",
        ru: "Проверить с ИИ после подтверждения" }
    /// AI review button once the confirmation is showing.
    confirm_send { en: "Confirm sending and review", pt: "Confirmar envio e revisar",
        es: "Confirmar envío y revisar", fr: "Confirmer l’envoi et revoir",
        de: "Senden bestätigen und prüfen", it: "Conferma l’invio e rivedi",
        ja: "送信を確定してレビュー", zh: "确认发送并审阅", ko: "전송 확인 후 검토",
        ru: "Подтвердить отправку и проверить" }
    /// AI review button.
    review_ai { en: "Review with AI…", pt: "Revisar com IA…", es: "Revisar con IA…",
        fr: "Revoir avec l’IA…", de: "Mit KI prüfen…", it: "Rivedi con IA…",
        ja: "AIでレビュー…", zh: "用 AI 审阅…", ko: "AI로 검토…", ru: "Проверить с ИИ…" }
    /// Aria label of the cancel button.
    cancel_aria { en: "Cancel review or confirmation", pt: "Cancelar revisão ou confirmação",
        es: "Cancelar revisión o confirmación", fr: "Annuler la revue ou la confirmation",
        de: "Prüfung oder Bestätigung abbrechen", it: "Annulla la revisione o la conferma",
        ja: "レビューまたは確認をキャンセル", zh: "取消审阅或确认",
        ko: "검토 또는 확인 취소", ru: "Отменить проверку или подтверждение" }
    /// Cancel button.
    cancel { en: "Cancel", pt: "Cancelar", es: "Cancelar", fr: "Annuler", de: "Abbrechen",
        it: "Annulla", ja: "キャンセル", zh: "取消", ko: "취소", ru: "Отмена" }
    /// Toast after the review finished.
    done_toast { en: "Review complete; no data changed.",
        pt: "Revisão concluída; nenhum dado alterado.",
        es: "Revisión completada; no se modificó ningún dato.",
        fr: "Revue terminée ; aucune donnée modifiée.",
        de: "Prüfung abgeschlossen; keine Daten geändert.",
        it: "Revisione completata; nessun dato modificato.",
        ja: "レビューが完了しました。データは変更されていません。",
        zh: "审阅完成；未更改任何数据。",
        ko: "검토를 마쳤어요. 변경된 데이터는 없어요.",
        ru: "Проверка завершена; данные не изменены." }
}

formats! {
    /// Inventory line of the local checks.
    inventory(current: usize, superseded: usize, rules: usize, relations: usize) {
        en: "{current} decisions in effect · {superseded} superseded · {rules} valid rules · {relations} relations",
        pt: "{current} decisões em vigor · {superseded} substituídas · {rules} regras válidas · {relations} relações",
        es: "{current} decisiones vigentes · {superseded} sustituidas · {rules} reglas válidas · {relations} relaciones",
        fr: "{current} décisions en vigueur · {superseded} remplacées · {rules} règles valides · {relations} relations",
        de: "{current} Entscheidungen in Kraft · {superseded} ersetzt · {rules} gültige Regeln · {relations} Beziehungen",
        it: "{current} decisioni in vigore · {superseded} sostituite · {rules} regole valide · {relations} relazioni",
        ja: "有効な決定 {current}件 · 置き換え済み {superseded}件 · 有効なルール {rules}件 · 関係 {relations}件",
        zh: "生效中的决策 {current} 项 · 已取代 {superseded} 项 · 有效规则 {rules} 条 · 关系 {relations} 个",
        ko: "유효한 결정 {current}개 · 대체됨 {superseded}개 · 유효한 규칙 {rules}개 · 관계 {relations}개",
        ru: "Действующих решений: {current} · заменено: {superseded} · действующих правил: {rules} · связей: {relations}" }
    /// Pending proposals line of the local checks.
    pending_proposals(relations: usize, rules: usize, links: usize) {
        en: "Pending proposals: {relations} relations · {rules} rules · {links} links",
        pt: "Propostas pendentes: {relations} relações · {rules} regras · {links} vínculos",
        es: "Propuestas pendientes: {relations} relaciones · {rules} reglas · {links} vínculos",
        fr: "Propositions en attente : {relations} relations · {rules} règles · {links} liens",
        de: "Offene Vorschläge: {relations} Beziehungen · {rules} Regeln · {links} Verknüpfungen",
        it: "Proposte in sospeso: {relations} relazioni · {rules} regole · {links} collegamenti",
        ja: "保留中の提案: 関係 {relations}件 · ルール {rules}件 · リンク {links}件",
        zh: "待处理的提案：关系 {relations} 个 · 规则 {rules} 条 · 关联 {links} 个",
        ko: "보류 중인 제안: 관계 {relations}개 · 규칙 {rules}개 · 연결 {links}개",
        ru: "Ожидающие предложения: связей — {relations} · правил — {rules} · ссылок — {links}" }
    /// Provider and model that wrote the AI review.
    provider_model(provider: &str, model: &str) {
        en: "Provider: {provider} · model: {model}", pt: "Provedor: {provider} · modelo: {model}",
        es: "Proveedor: {provider} · modelo: {model}", fr: "Fournisseur : {provider} · modèle : {model}",
        de: "Anbieter: {provider} · Modell: {model}", it: "Provider: {provider} · modello: {model}",
        ja: "プロバイダー: {provider} · モデル: {model}", zh: "提供商：{provider} · 模型：{model}",
        ko: "공급자: {provider} · 모델: {model}", ru: "Провайдер: {provider} · модель: {model}" }
    /// Where a piece of evidence comes from.
    evidence_meta(title: &str, version: &str, field: &str) {
        en: "{title} · version {version} · field {field}", pt: "{title} · versão {version} · campo {field}",
        es: "{title} · versión {version} · campo {field}", fr: "{title} · version {version} · champ {field}",
        de: "{title} · Version {version} · Feld {field}", it: "{title} · versione {version} · campo {field}",
        ja: "{title} · バージョン {version} · フィールド {field}", zh: "{title} · 版本 {version} · 字段 {field}",
        ko: "{title} · 버전 {version} · 필드 {field}", ru: "{title} · версия {version} · поле {field}" }
    /// Summary of the coverage.
    coverage_summary(units: usize, discarded: usize) {
        en: "{units} units selected · {discarded} findings discarded",
        pt: "{units} unidades selecionadas · {discarded} achados descartados",
        es: "{units} unidades seleccionadas · {discarded} hallazgos descartados",
        fr: "{units} unités sélectionnées · {discarded} constats écartés",
        de: "{units} Einheiten ausgewählt · {discarded} Befunde verworfen",
        it: "{units} unità selezionate · {discarded} riscontri scartati",
        ja: "選択した単位 {units}件 · 破棄した検出結果 {discarded}件",
        zh: "已选 {units} 个单元 · 已丢弃 {discarded} 条发现",
        ko: "선택한 단위 {units}개 · 버린 결과 {discarded}개",
        ru: "Выбрано единиц: {units} · отброшено находок: {discarded}" }
    /// A part of the knowledge the review left out, and why.
    not_reviewed(reason: &str, subjects: &str) {
        en: "Not reviewed: {reason} · {subjects}", pt: "Não revisado: {reason} · {subjects}",
        es: "No revisado: {reason} · {subjects}", fr: "Non revu : {reason} · {subjects}",
        de: "Nicht geprüft: {reason} · {subjects}", it: "Non rivisto: {reason} · {subjects}",
        ja: "未レビュー: {reason} · {subjects}", zh: "未审阅：{reason} · {subjects}",
        ko: "검토하지 않음: {reason} · {subjects}", ru: "Не проверено: {reason} · {subjects}" }
}
