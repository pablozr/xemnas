//! Copy for the review ticket (one decision per screen). See [`crate::i18n`]
//! for how entries are declared.

use super::{formats, strings};

strings! {
    /// Before the number of safe candidates the AI accepted (Automatic mode).
    ai_accepted_before { en: "The AI accepted", pt: "A IA aceitou", es: "La IA aceptó",
        fr: "L’IA a accepté", de: "Die KI hat", it: "L’IA ha accettato", ja: "AI が承認:",
        zh: "AI 已接受", ko: "AI가 수락한 안전한 항목", ru: "ИИ принял" }
    /// After that number.
    ai_accepted_after { en: "safe ones", pt: "seguras", es: "seguras", fr: "sûres",
        de: "sichere übernommen", it: "sicure", ja: "件", zh: "项安全决定", ko: "개",
        ru: "безопасных" }
    /// Link that opens the list of what the AI accepted (key L).
    see { en: "see", pt: "ver", es: "ver", fr: "voir", de: "ansehen", it: "vedi",
        ja: "見る", zh: "查看", ko: "보기", ru: "смотреть" }
    /// Manual mode: the person decides every candidate.
    you_decide_all { en: "You decide every one", pt: "Você decide todas",
        es: "Tú decides todas", fr: "Vous décidez de tout", de: "Du entscheidest alles",
        it: "Decidi tu tutte", ja: "すべてあなたが決めます", zh: "全部由你决定",
        ko: "모두 직접 결정해요", ru: "Вы решаете все" }
    /// Hint of the key that opens the queue.
    queue_hint { en: "queue", pt: "fila", es: "cola", fr: "file", de: "Warteschlange",
        it: "coda", ja: "キュー", zh: "队列", ko: "대기열", ru: "очередь" }
    /// Name of the queue overlay for assistive technology.
    queue_overlay_aria { en: "Review queue", pt: "Fila de revisão", es: "Cola de revisión",
        fr: "File de revue", de: "Prüfwarteschlange", it: "Coda di revisione",
        ja: "レビューキュー", zh: "审阅队列", ko: "검토 대기열", ru: "Очередь проверки" }
    /// Caption under the old value of the ticket.
    before { en: "before", pt: "antes", es: "antes", fr: "avant", de: "vorher",
        it: "prima", ja: "変更前", zh: "之前", ko: "이전", ru: "было" }
    /// Caption under the new value of the ticket.
    now { en: "now", pt: "agora", es: "ahora", fr: "maintenant", de: "jetzt",
        it: "ora", ja: "変更後", zh: "现在", ko: "현재", ru: "стало" }
    /// Ticket field: how far the choice reaches.
    field_scope { en: "SCOPE", pt: "ESCOPO", es: "ALCANCE", fr: "PORTÉE", de: "BEREICH",
        it: "AMBITO", ja: "範囲", zh: "范围", ko: "범위", ru: "ОБЛАСТЬ" }
    /// Ticket field: the project it was seen in.
    field_where { en: "WHERE", pt: "ONDE", es: "DÓNDE", fr: "OÙ", de: "WO",
        it: "DOVE", ja: "場所", zh: "位置", ko: "위치", ru: "ГДЕ" }
    /// Ticket field: what kind of item it becomes.
    field_applies { en: "APPLIES TO", pt: "VALE PARA", es: "VALE PARA", fr: "VAUT POUR",
        de: "GILT FÜR", it: "VALE PER", ja: "対象", zh: "适用于", ko: "적용 대상",
        ru: "ПРИМЕНЯЕТСЯ К" }
    /// Ticket field: what it replaces.
    field_replaces { en: "REPLACES", pt: "SUBSTITUI", es: "SUSTITUYE", fr: "REMPLACE",
        de: "ERSETZT", it: "SOSTITUISCE", ja: "置換", zh: "替换", ko: "대체", ru: "ЗАМЕНЯЕТ" }
    /// Nothing known for a ticket field.
    field_none { en: "—", pt: "—", es: "—", fr: "—", de: "—", it: "—", ja: "—", zh: "—",
        ko: "—", ru: "—" }
    /// Confidence level in the stub.
    confidence_high { en: "high", pt: "alta", es: "alta", fr: "élevée", de: "hoch",
        it: "alta", ja: "高", zh: "高", ko: "높음", ru: "высокая" }
    /// Confidence level in the stub.
    confidence_medium { en: "medium", pt: "média", es: "media", fr: "moyenne",
        de: "mittel", it: "media", ja: "中", zh: "中", ko: "보통", ru: "средняя" }
    /// Confidence level in the stub.
    confidence_low { en: "low", pt: "baixa", es: "baja", fr: "faible", de: "niedrig",
        it: "bassa", ja: "低", zh: "低", ko: "낮음", ru: "низкая" }
    /// Opens the proof (key Enter).
    see_proof { en: "see the proof", pt: "ver a prova", es: "ver la prueba",
        fr: "voir la preuve", de: "Beleg ansehen", it: "vedi la prova", ja: "根拠を見る",
        zh: "查看依据", ko: "근거 보기", ru: "смотреть доказательство" }
    /// Opens what changes if the candidate is confirmed (key E).
    see_effect { en: "what changes if you confirm", pt: "o que muda se confirmar",
        es: "qué cambia si confirmas", fr: "ce qui change si vous confirmez",
        de: "was sich bei Bestätigung ändert", it: "cosa cambia se confermi",
        ja: "確定すると何が変わるか", zh: "确认后会发生什么", ko: "확정하면 바뀌는 것",
        ru: "что изменится при подтверждении" }
    /// Title of the proof view.
    proof_title { en: "Today’s review", pt: "Revisão de hoje", es: "Revisión de hoy",
        fr: "Revue du jour", de: "Prüfung von heute", it: "Revisione di oggi",
        ja: "今日のレビュー", zh: "今日审阅", ko: "오늘의 검토", ru: "Проверка за сегодня" }
    /// Label of the quoted passage that supports the choice.
    proof_why { en: "WHY", pt: "POR QUE", es: "POR QUÉ", fr: "POURQUOI", de: "WARUM",
        it: "PERCHÉ", ja: "理由", zh: "原因", ko: "이유", ru: "ПОЧЕМУ" }
    /// Hint to close the proof or effect view.
    close_hint { en: "close", pt: "fechar", es: "cerrar", fr: "fermer", de: "schließen",
        it: "chiudi", ja: "閉じる", zh: "关闭", ko: "닫기", ru: "закрыть" }
    /// Hint to move between sources in the proof.
    other_source_hint { en: "other source", pt: "outra fonte", es: "otra fuente",
        fr: "autre source", de: "andere Quelle", it: "altra fonte", ja: "別の出典",
        zh: "其他来源", ko: "다른 출처", ru: "другой источник" }
    /// Title of the effect view.
    effect_title { en: "What changes if you confirm", pt: "O que muda se confirmar",
        es: "Qué cambia si confirmas", fr: "Ce qui change si vous confirmez",
        de: "Was sich bei Bestätigung ändert", it: "Cosa cambia se confermi",
        ja: "確定すると何が変わるか", zh: "确认后会发生什么", ko: "확정하면 바뀌는 것",
        ru: "Что изменится при подтверждении" }
    /// Tag on the ticket the AI left to the person; the reason follows.
    left_tag { en: "the AI left this to you", pt: "a IA deixou para você",
        es: "la IA lo dejó para ti", fr: "l’IA vous l’a laissé",
        de: "die KI hat es dir überlassen", it: "l’IA l’ha lasciato a te",
        ja: "AI があなたに任せました", zh: "AI 留给了你", ko: "AI가 맡겼어요",
        ru: "ИИ оставил это вам" }
    /// Title of the empty queue.
    zero_title { en: "Queue cleared.", pt: "Fila zerada.", es: "Cola vacía.",
        fr: "File vidée.", de: "Warteschlange leer.", it: "Coda azzerata.",
        ja: "キューは空です。", zh: "队列已清空。", ko: "대기열이 비었어요.",
        ru: "Очередь пуста." }
    /// Shortcut on the empty queue: today’s decisions (key D).
    zero_decisions { en: "see today’s decisions", pt: "ver as decisões de hoje",
        es: "ver las decisiones de hoy", fr: "voir les décisions du jour",
        de: "Entscheidungen von heute ansehen", it: "vedi le decisioni di oggi",
        ja: "今日の決定を見る", zh: "查看今日决定", ko: "오늘의 결정 보기",
        ru: "решения за сегодня" }
    /// Shortcut on the empty queue: what the AI accepted (key L).
    zero_accepted { en: "what the AI accepted", pt: "o que a IA aceitou",
        es: "lo que aceptó la IA", fr: "ce que l’IA a accepté", de: "was die KI übernommen hat",
        it: "cosa ha accettato l’IA", ja: "AI が承認したもの", zh: "AI 接受的内容",
        ko: "AI가 수락한 항목", ru: "что принял ИИ" }
    /// Right side of the colophon under the night scene.
    colophon_night { en: "night · every star in the arc is a decision from today · made with GPUI",
        pt: "noturno · cada estrela do arco é uma decisão de hoje · feito com GPUI",
        es: "nocturno · cada estrella del arco es una decisión de hoy · hecho con GPUI",
        fr: "nocturne · chaque étoile de l’arc est une décision du jour · fait avec GPUI",
        de: "Nachtbild · jeder Stern im Bogen ist eine Entscheidung von heute · gemacht mit GPUI",
        it: "notturno · ogni stella dell’arco è una decisione di oggi · fatto con GPUI",
        ja: "夜景 · 弧の星は今日の決定ひとつずつ · GPUI で制作",
        zh: "夜景 · 弧线上的每颗星都是今天的一个决定 · 使用 GPUI 制作",
        ko: "야경 · 호의 별 하나가 오늘의 결정 하나 · GPUI로 제작",
        ru: "ночь · каждая звезда дуги — решение за сегодня · сделано на GPUI" }
    /// Right side of the colophon when the queue is cleared.
    colophon_closed { en: "night · the arc closed: ten stars, ten decisions · made with GPUI",
        pt: "noturno · o arco fechou: dez estrelas, dez decisões · feito com GPUI",
        es: "nocturno · el arco se cerró: diez estrellas, diez decisiones · hecho con GPUI",
        fr: "nocturne · l’arc est fermé : dix étoiles, dix décisions · fait avec GPUI",
        de: "Nachtbild · der Bogen ist geschlossen: zehn Sterne, zehn Entscheidungen · gemacht mit GPUI",
        it: "notturno · l’arco si è chiuso: dieci stelle, dieci decisioni · fatto con GPUI",
        ja: "夜景 · 弧が閉じました: 星十個、決定十件 · GPUI で制作",
        zh: "夜景 · 弧线已闭合：十颗星，十个决定 · 使用 GPUI 制作",
        ko: "야경 · 호가 닫혔어요: 별 열 개, 결정 열 개 · GPUI로 제작",
        ru: "ночь · дуга замкнулась: десять звёзд, десять решений · сделано на GPUI" }
}

formats! {
    /// Position among the decisions of the cycle: “3 of 10”.
    position(index: usize, total: usize) { en: "{index} of {total}", pt: "{index} de {total}",
        es: "{index} de {total}", fr: "{index} sur {total}", de: "{index} von {total}",
        it: "{index} su {total}", ja: "{index}/{total}", zh: "{index}/{total}",
        ko: "{index}/{total}", ru: "{index} из {total}" }
    /// Automatic mode: how many of the cycle are for the person.
    for_you_of(index: usize, total: usize) { en: "{index} of {total} for you",
        pt: "{index} de {total} para você", es: "{index} de {total} para ti",
        fr: "{index} sur {total} pour vous", de: "{index} von {total} für dich",
        it: "{index} su {total} per te", ja: "あなたの分 {index}/{total}",
        zh: "留给你的 {index}/{total}", ko: "내 몫 {index}/{total}",
        ru: "{index} из {total} для вас" }
    /// The next ticket, under the action pill.
    next_up(question: &str) { en: "Next · {question}", pt: "A seguir · {question}",
        es: "Siguiente · {question}", fr: "À suivre · {question}", de: "Als Nächstes · {question}",
        it: "Poi · {question}", ja: "次 · {question}", zh: "下一个 · {question}",
        ko: "다음 · {question}", ru: "Далее · {question}" }
    /// Confidence level and number of sources in the stub: “high · 3 sources”.
    confidence_line(level: &str, sources: usize) { en: "{level} · {sources} sources",
        pt: "{level} · {sources} fontes", es: "{level} · {sources} fuentes",
        fr: "{level} · {sources} sources", de: "{level} · {sources} Quellen",
        it: "{level} · {sources} fonti", ja: "{level} · 出典 {sources}",
        zh: "{level} · {sources} 个来源", ko: "{level} · 출처 {sources}개",
        ru: "{level} · источников: {sources}" }
    /// Totals of the day on the empty queue.
    zero_summary(total: usize, ai: usize, you: usize) {
        en: "{total} decisions today · {ai} by the AI · {you} by you",
        pt: "{total} decisões hoje · {ai} pela IA · {you} por você",
        es: "{total} decisiones hoy · {ai} por la IA · {you} por ti",
        fr: "{total} décisions aujourd’hui · {ai} par l’IA · {you} par vous",
        de: "{total} Entscheidungen heute · {ai} durch die KI · {you} durch dich",
        it: "{total} decisioni oggi · {ai} dall’IA · {you} da te",
        ja: "今日の決定 {total} 件 · AI {ai} 件 · あなた {you} 件",
        zh: "今天 {total} 个决定 · AI {ai} 个 · 你 {you} 个",
        ko: "오늘 결정 {total}건 · AI {ai}건 · 직접 {you}건",
        ru: "Сегодня решений: {total} · ИИ: {ai} · вы: {you}" }
    /// Left side of the colophon: version, commit and licence.
    colophon_build(version: &str, commit: &str, licence: &str) {
        en: "xemnas {version} · {commit} · {licence}", pt: "xemnas {version} · {commit} · {licence}",
        es: "xemnas {version} · {commit} · {licence}", fr: "xemnas {version} · {commit} · {licence}",
        de: "xemnas {version} · {commit} · {licence}", it: "xemnas {version} · {commit} · {licence}",
        ja: "xemnas {version} · {commit} · {licence}", zh: "xemnas {version} · {commit} · {licence}",
        ko: "xemnas {version} · {commit} · {licence}", ru: "xemnas {version} · {commit} · {licence}" }
}
