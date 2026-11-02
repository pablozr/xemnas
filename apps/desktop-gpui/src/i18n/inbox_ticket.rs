//! Copy for the review ticket (one decision per screen). See [`crate::i18n`]
//! for how entries are declared.

use super::common::counted;
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
    /// Caption above the proposed value when there is no earlier one.
    new_decision { en: "NEW DECISION", pt: "NOVA DECISÃO", es: "NUEVA DECISIÓN",
        fr: "NOUVELLE DÉCISION", de: "NEUE ENTSCHEIDUNG", it: "NUOVA DECISIONE",
        ja: "新しい決定", zh: "新决定", ko: "새 결정", ru: "НОВОЕ РЕШЕНИЕ" }
    /// Caption above the number in the stub.
    confidence_label { en: "CONFIDENCE", pt: "CONFIANÇA", es: "CONFIANZA",
        fr: "CONFIANCE", de: "SICHERHEIT", it: "FIDUCIA", ja: "確信度", zh: "置信度",
        ko: "신뢰도", ru: "УВЕРЕННОСТЬ" }
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
    /// Title of the empty queue when nobody decided anything today.
    empty_title_sentence { en: "Nothing waiting for review.", pt: "Nada aguardando revisão.",
        es: "Nada pendiente de revisión.", fr: "Rien en attente de revue.",
        de: "Nichts wartet auf Prüfung.", it: "Niente in attesa di revisione.",
        ja: "レビュー待ちはありません。", zh: "没有等待审阅的内容。",
        ko: "검토 대기 중인 항목이 없어요.", ru: "Нет ничего на проверке." }
    /// Capture state B: listening, nothing loaded yet.
    capture_none_title { en: "Capture active · no session has arrived yet.",
        pt: "Captura ativa · nenhuma sessão chegou ainda.",
        es: "Captura activa · todavía no llegó ninguna sesión.",
        fr: "Capture active · aucune session n’est encore arrivée.",
        de: "Erfassung aktiv · noch keine Sitzung angekommen.",
        it: "Acquisizione attiva · non è ancora arrivata nessuna sessione.",
        ja: "キャプチャ有効 · まだセッションが届いていません。",
        zh: "捕获已开启 · 还没有会话到达。",
        ko: "캡처 켜짐 · 아직 도착한 세션이 없어요.",
        ru: "Захват включён · сессий пока не было." }
    /// Second line of state B.
    capture_none_body { en: "Work with your agent in this project and its choices show up here.",
        pt: "Trabalhe com seu agente neste projeto e as escolhas aparecem aqui.",
        es: "Trabaja con tu agente en este proyecto y sus elecciones aparecerán aquí.",
        fr: "Travaillez avec votre agent dans ce projet et ses choix apparaîtront ici.",
        de: "Arbeite mit deinem Agenten in diesem Projekt, dann erscheinen seine Entscheidungen hier.",
        it: "Lavora con il tuo agente in questo progetto e le sue scelte compariranno qui.",
        ja: "このプロジェクトでエージェントと作業すると、その選択がここに表示されます。",
        zh: "在此项目中与你的智能体协作，它的选择会显示在这里。",
        ko: "이 프로젝트에서 에이전트와 작업하면 선택이 여기에 나타나요.",
        ru: "Работайте с агентом в этом проекте, и его решения появятся здесь." }
    /// Second line of state C (listening, but nothing for a day or more).
    capture_stale_body { en: "The app’s capture is on; check that your agent is connected.",
        pt: "A captura do app está ativa; confira se o agente está conectado.",
        es: "La captura de la app está activa; comprueba que tu agente esté conectado.",
        fr: "La capture de l’app est active ; vérifiez que votre agent est connecté.",
        de: "Die Erfassung der App ist aktiv; prüfe, ob dein Agent verbunden ist.",
        it: "L’acquisizione dell’app è attiva; controlla che il tuo agente sia connesso.",
        ja: "アプリのキャプチャは有効です。エージェントが接続されているか確認してください。",
        zh: "应用的捕获已开启；请检查你的智能体是否已连接。",
        ko: "앱의 캡처는 켜져 있어요. 에이전트가 연결되어 있는지 확인하세요.",
        ru: "Захват в приложении включён; проверьте, что агент подключён." }
    /// Title of state D (the local API did not start).
    capture_stopped_title { en: "Capture is stopped.", pt: "A captura está parada.",
        es: "La captura está detenida.", fr: "La capture est arrêtée.",
        de: "Die Erfassung ist angehalten.", it: "L’acquisizione è ferma.",
        ja: "キャプチャは停止しています。", zh: "捕获已停止。", ko: "캡처가 멈췄어요.",
        ru: "Захват остановлен." }
    /// Second line of state D.
    capture_stopped_body {
        en: "The local API didn’t start. Sessions wait in the agent and arrive when it’s back.",
        pt: "A API local não iniciou. As sessões ficam guardadas no agente e chegam quando ela voltar.",
        es: "La API local no se inició. Las sesiones esperan en el agente y llegan cuando vuelva.",
        fr: "L’API locale n’a pas démarré. Les sessions attendent dans l’agent et arriveront à son retour.",
        de: "Die lokale API ist nicht gestartet. Sitzungen warten im Agenten und kommen an, sobald sie wieder läuft.",
        it: "L’API locale non si è avviata. Le sessioni restano nell’agente e arrivano quando torna.",
        ja: "ローカル API が起動しませんでした。セッションはエージェントに保持され、復帰すると届きます。",
        zh: "本地 API 未启动。会话保存在智能体中，恢复后会送达。",
        ko: "로컬 API가 시작되지 않았어요. 세션은 에이전트에 보관되고 다시 켜지면 도착해요.",
        ru: "Локальный API не запустился. Сессии ждут в агенте и придут, когда он вернётся." }
    /// Capture state F (demo).
    capture_demo_line { en: "Demo · fictional data, no real capture",
        pt: "Demonstração · dados fictícios, nenhuma captura real",
        es: "Demostración · datos ficticios, ninguna captura real",
        fr: "Démonstration · données fictives, aucune capture réelle",
        de: "Demo · fiktive Daten, keine echte Erfassung",
        it: "Demo · dati fittizi, nessuna acquisizione reale",
        ja: "デモ · 架空のデータ、実際のキャプチャはありません",
        zh: "演示 · 虚构数据，没有真实捕获", ko: "데모 · 가상 데이터, 실제 캡처 없음",
        ru: "Демо · вымышленные данные, без настоящего захвата" }
    /// Opens Settings on Diagnostics from a capture warning.
    open_diagnostics { en: "View diagnostics", pt: "Ver diagnóstico", es: "Ver diagnóstico",
        fr: "Voir le diagnostic", de: "Diagnose ansehen", it: "Vedi diagnostica",
        ja: "診断を見る", zh: "查看诊断", ko: "진단 보기", ru: "Открыть диагностику" }
    /// Project bar pill in state D.
    capture_pill_stopped { en: "Capture stopped", pt: "Captura parada",
        es: "Captura detenida", fr: "Capture arrêtée", de: "Erfassung angehalten",
        it: "Acquisizione ferma", ja: "キャプチャ停止", zh: "捕获已停止", ko: "캡처 멈춤",
        ru: "Захват остановлен" }
    /// Project bar pill in state C.
    capture_pill_stale { en: "Nothing arriving", pt: "Nada chega", es: "No llega nada",
        fr: "Rien n’arrive", de: "Nichts kommt an", it: "Non arriva nulla",
        ja: "何も届いていません", zh: "没有新内容", ko: "도착한 것 없음", ru: "Ничего не приходит" }
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
    /// Capture state A: listening, last capture less than a day ago.
    capture_ok(when: &str) { en: "Capture active · last capture: {when}",
        pt: "Captura ativa · última captura: {when}",
        es: "Captura activa · última captura: {when}",
        fr: "Capture active · dernière capture : {when}",
        de: "Erfassung aktiv · letzte Erfassung: {when}",
        it: "Acquisizione attiva · ultima acquisizione: {when}",
        ja: "キャプチャ有効 · 最後のキャプチャ: {when}",
        zh: "捕获已开启 · 最近一次捕获：{when}", ko: "캡처 켜짐 · 마지막 캡처: {when}",
        ru: "Захват включён · последний захват: {when}" }
    /// Title of state C: nothing arrived for a day or more.
    capture_stale_title(when: &str) { en: "Nothing has arrived for {when}.",
        pt: "Nada chega há {when}.", es: "No llega nada desde hace {when}.",
        fr: "Rien n’est arrivé depuis {when}.", de: "Seit {when} ist nichts angekommen.",
        it: "Non arriva nulla da {when}.", ja: "{when} 何も届いていません。",
        zh: "已有 {when} 没有新内容。", ko: "{when} 동안 도착한 것이 없어요.",
        ru: "Ничего не приходит уже {when}." }
}

/// Captures of the project whose analysis failed (state E of the empty queue).
pub fn capture_not_analysed(n: usize) -> String {
    counted(
        n,
        [
            [
                "capture wasn’t analysed",
                "captures weren’t analysed",
                "captures weren’t analysed",
            ],
            [
                "captura não foi analisada",
                "capturas não foram analisadas",
                "capturas não foram analisadas",
            ],
            [
                "captura no se analizó",
                "capturas no se analizaron",
                "capturas no se analizaron",
            ],
            [
                "capture n’a pas été analysée",
                "captures n’ont pas été analysées",
                "captures n’ont pas été analysées",
            ],
            [
                "Erfassung wurde nicht analysiert",
                "Erfassungen wurden nicht analysiert",
                "Erfassungen wurden nicht analysiert",
            ],
            [
                "acquisizione non è stata analizzata",
                "acquisizioni non sono state analizzate",
                "acquisizioni non sono state analizzate",
            ],
            [
                "件のキャプチャが未分析です",
                "件のキャプチャが未分析です",
                "件のキャプチャが未分析です",
            ],
            ["个捕获未被分析", "个捕获未被分析", "个捕获未被分析"],
            [
                "개의 캡처가 분석되지 않았어요",
                "개의 캡처가 분석되지 않았어요",
                "개의 캡처가 분석되지 않았어요",
            ],
            [
                "захват не проанализирован",
                "захвата не проанализированы",
                "захватов не проанализированы",
            ],
        ],
    )
}

/// Project bar pill in state E: failed analyses of the project's captures.
pub fn capture_pill_failed(n: usize) -> String {
    counted(
        n,
        [
            ["failure", "failures", "failures"],
            ["falha", "falhas", "falhas"],
            ["fallo", "fallos", "fallos"],
            ["échec", "échecs", "échecs"],
            ["Fehler", "Fehler", "Fehler"],
            ["errore", "errori", "errori"],
            ["件の失敗", "件の失敗", "件の失敗"],
            ["个失败", "个失败", "个失败"],
            ["건 실패", "건 실패", "건 실패"],
            ["сбой", "сбоя", "сбоев"],
        ],
    )
}
