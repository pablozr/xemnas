//! Copy for the graph area. See [`crate::i18n`] for how entries are declared.

use super::{formats, french_one, one, russian_form, strings, Language};

formats! {
    /// Name of a group node folding `count` decisions.
    group_decisions_label(count: usize) {
        en: "+{count} decisions", pt: "+{count} decisões", es: "+{count} decisiones",
        fr: "+{count} décisions", de: "+{count} Entscheidungen",
        it: "+{count} decisioni", ja: "+{count}件の決定", zh: "+{count} 项决策",
        ko: "+{count}개 결정", ru: "+{count} решений" }
    /// Name of a group node folding `count` rules.
    group_rules_label(count: usize) {
        en: "+{count} rules", pt: "+{count} regras", es: "+{count} reglas",
        fr: "+{count} règles", de: "+{count} Regeln", it: "+{count} regole",
        ja: "+{count}件のルール", zh: "+{count} 条规则", ko: "+{count}개 규칙",
        ru: "+{count} правил" }
    /// Card caption of a conflicting node.
    caption_conflict(caption: &str) {
        en: "{caption} · in conflict", pt: "{caption} · em conflito",
        es: "{caption} · en conflicto", fr: "{caption} · en conflit",
        de: "{caption} · im Konflikt", it: "{caption} · in conflitto",
        ja: "{caption} · 競合中", zh: "{caption} · 存在冲突", ko: "{caption} · 충돌 중",
        ru: "{caption} · конфликт" }
    /// Screen-reader state of a layer filter that is on.
    layer_visible(label: &str) {
        en: "{label}: shown", pt: "{label}: visíveis", es: "{label}: visibles",
        fr: "{label} : affichés", de: "{label}: sichtbar", it: "{label}: visibili",
        ja: "{label}：表示中", zh: "{label}：显示中", ko: "{label}: 표시 중",
        ru: "{label}: показаны" }
    /// Screen-reader state of a layer filter that is off.
    layer_hidden(label: &str) {
        en: "{label}: hidden", pt: "{label}: ocultas", es: "{label}: ocultas",
        fr: "{label} : masqués", de: "{label}: ausgeblendet", it: "{label}: nascoste",
        ja: "{label}：非表示", zh: "{label}：已隐藏", ko: "{label}: 숨김",
        ru: "{label}: скрыты" }
    /// Question on a suggested link to `name`.
    link_to(name: &str) {
        en: "Link to {name}?", pt: "Ligar a {name}?", es: "¿Vincular con {name}?",
        fr: "Lier à {name} ?", de: "Mit {name} verknüpfen?", it: "Collegare a {name}?",
        ja: "{name}とリンクしますか？", zh: "要关联到 {name} 吗？", ko: "{name}에 연결할까요?",
        ru: "Связать с «{name}»?" }
    /// How many more ties the card does not list.
    and_more(more: usize) {
        en: "And {more} more", pt: "E mais {more}", es: "Y {more} más",
        fr: "Et {more} de plus", de: "Und {more} weitere", it: "E altri {more}",
        ja: "ほか{more}件", zh: "还有 {more} 项", ko: "외 {more}개", ru: "И ещё {more}" }
    /// Hint while the map is zoomed out and nodes are hidden.
    hint_panorama(omitted: usize) {
        en: "Overview · {omitted} decisions and rules hidden · zoom in to see them",
        pt: "Panorama · {omitted} decisões e regras ocultas · aproxime para vê-las",
        es: "Panorama · {omitted} decisiones y reglas ocultas · acércate para verlas",
        fr: "Panorama · {omitted} décisions et règles masquées · zoomez pour les voir",
        de: "Überblick · {omitted} Entscheidungen und Regeln ausgeblendet · zoome hinein, um sie zu sehen",
        it: "Panoramica · {omitted} decisioni e regole nascoste · ingrandisci per vederle",
        ja: "全体表示 · 決定とルール{omitted}件を非表示 · ズームインすると表示されます",
        zh: "全景 · 已隐藏 {omitted} 项决策和规则 · 放大即可查看",
        ko: "전체 보기 · 결정과 규칙 {omitted}개 숨김 · 확대하면 볼 수 있어요",
        ru: "Обзор · скрыто решений и правил: {omitted} · приблизьте, чтобы увидеть" }
    /// Screen-reader summary of the whole graph.
    summary(components: usize, decisions: usize, rules: usize, technologies: usize) {
        en: "Project graph: {components} components, {decisions} decisions, {rules} rules, {technologies} technologies.",
        pt: "Grafo do projeto: {components} componentes, {decisions} decisões, {rules} regras, {technologies} tecnologias.",
        es: "Grafo del proyecto: {components} componentes, {decisions} decisiones, {rules} reglas, {technologies} tecnologías.",
        fr: "Graphe du projet : {components} composants, {decisions} décisions, {rules} règles, {technologies} technologies.",
        de: "Projekt-Graph: {components} Komponenten, {decisions} Entscheidungen, {rules} Regeln, {technologies} Technologien.",
        it: "Grafo del progetto: {components} componenti, {decisions} decisioni, {rules} regole, {technologies} tecnologie.",
        ja: "プロジェクトのグラフ：コンポーネント{components}件、決定{decisions}件、ルール{rules}件、テクノロジー{technologies}件。",
        zh: "项目图谱：{components} 个组件、{decisions} 项决策、{rules} 条规则、{technologies} 项技术。",
        ko: "프로젝트 그래프: 컴포넌트 {components}개, 결정 {decisions}개, 규칙 {rules}개, 기술 {technologies}개.",
        ru: "Граф проекта: компонентов — {components}, решений — {decisions}, правил — {rules}, технологий — {technologies}." }
}

strings! {
    /// Node kind: component.
    kind_component { en: "Component", pt: "Componente", es: "Componente", fr: "Composant",
        de: "Komponente", it: "Componente", ja: "コンポーネント", zh: "组件",
        ko: "컴포넌트", ru: "Компонент" }
    /// Node kind: technology.
    kind_technology { en: "Technology", pt: "Tecnologia", es: "Tecnología",
        fr: "Technologie", de: "Technologie", it: "Tecnologia", ja: "テクノロジー",
        zh: "技术", ko: "기술", ru: "Технология" }
    /// Node kind: decision.
    kind_decision { en: "Decision", pt: "Decisão", es: "Decisión", fr: "Décision",
        de: "Entscheidung", it: "Decisione", ja: "決定", zh: "决策", ko: "결정",
        ru: "Решение" }
    /// Node kind: rule.
    kind_rule { en: "Rule", pt: "Regra", es: "Regla", fr: "Règle", de: "Regel",
        it: "Regola", ja: "ルール", zh: "规则", ko: "규칙", ru: "Правило" }
    /// Rule kind: assumption.
    rule_assumption { en: "Assumption", pt: "Premissa", es: "Supuesto", fr: "Hypothèse",
        de: "Annahme", it: "Presupposto", ja: "前提", zh: "前提", ko: "전제",
        ru: "Допущение" }
    /// Rule kind: constraint.
    rule_constraint { en: "Constraint", pt: "Restrição", es: "Restricción",
        fr: "Contrainte", de: "Einschränkung", it: "Vincolo", ja: "制約", zh: "约束",
        ko: "제약", ru: "Ограничение" }
    /// Rule kind: goal.
    rule_goal { en: "Goal", pt: "Objetivo", es: "Objetivo", fr: "Objectif", de: "Ziel",
        it: "Obiettivo", ja: "目標", zh: "目标", ko: "목표", ru: "Цель" }
    /// Rule kind: convention.
    rule_convention { en: "Convention", pt: "Convenção", es: "Convención",
        fr: "Convention", de: "Konvention", it: "Convenzione", ja: "規約", zh: "约定",
        ko: "컨벤션", ru: "Соглашение" }
    /// Caption of a group node folding rules.
    caption_rules_grouped { en: "Grouped rules", pt: "Regras agrupadas",
        es: "Reglas agrupadas", fr: "Règles regroupées", de: "Gruppierte Regeln",
        it: "Regole raggruppate", ja: "まとめたルール", zh: "已分组的规则",
        ko: "묶인 규칙", ru: "Сгруппированные правила" }
    /// Caption of a group node folding decisions.
    caption_decisions_grouped { en: "Grouped decisions", pt: "Decisões agrupadas",
        es: "Decisiones agrupadas", fr: "Décisions regroupées",
        de: "Gruppierte Entscheidungen", it: "Decisioni raggruppate",
        ja: "まとめた決定", zh: "已分组的决策", ko: "묶인 결정",
        ru: "Сгруппированные решения" }
    /// Detail of a group node.
    group_detail { en: "Grouped to keep the graph light. Open the component to see them all.",
        pt: "Agrupadas para manter o grafo leve. Abra o componente para ver todas.",
        es: "Agrupadas para mantener el grafo ligero. Abre el componente para verlas todas.",
        fr: "Regroupées pour garder le graphe léger. Ouvrez le composant pour tout voir.",
        de: "Gruppiert, damit der Graph leicht bleibt. Öffne die Komponente, um alle zu sehen.",
        it: "Raggruppate per mantenere leggero il grafo. Apri il componente per vederle tutte.",
        ja: "グラフを軽く保つためにまとめています。コンポーネントを開くとすべて表示されます。",
        zh: "为保持图谱轻量而分组。打开组件即可查看全部。",
        ko: "그래프를 가볍게 유지하려고 묶었어요. 컴포넌트를 열면 모두 볼 수 있어요.",
        ru: "Сгруппированы, чтобы граф оставался лёгким. Откройте компонент, чтобы увидеть все." }
    /// Layer filter: decisions.
    chip_decisions { en: "Decisions", pt: "Decisões", es: "Decisiones", fr: "Décisions",
        de: "Entscheidungen", it: "Decisioni", ja: "決定", zh: "决策", ko: "결정",
        ru: "Решения" }
    /// Layer filter: rules.
    chip_rules { en: "Rules", pt: "Regras", es: "Reglas", fr: "Règles", de: "Regeln",
        it: "Regole", ja: "ルール", zh: "规则", ko: "규칙", ru: "Правила" }
    /// Layer filter: technologies.
    chip_technologies { en: "Technologies", pt: "Tecnologias", es: "Tecnologías",
        fr: "Technologies", de: "Technologien", it: "Tecnologie", ja: "テクノロジー",
        zh: "技术", ko: "기술", ru: "Технологии" }
    /// Layer filter: suggestions; also the card section of suggested links.
    chip_suggestions { en: "Suggestions", pt: "Sugestões", es: "Sugerencias",
        fr: "Suggestions", de: "Vorschläge", it: "Suggerimenti", ja: "提案", zh: "建议",
        ko: "제안", ru: "Предложения" }
    /// Zoom in button.
    zoom_in { en: "Zoom in", pt: "Aproximar", es: "Acercar", fr: "Zoom avant",
        de: "Vergrößern", it: "Ingrandisci", ja: "拡大", zh: "放大", ko: "확대",
        ru: "Приблизить" }
    /// Zoom out button.
    zoom_out { en: "Zoom out", pt: "Afastar", es: "Alejar", fr: "Zoom arrière",
        de: "Verkleinern", it: "Riduci", ja: "縮小", zh: "缩小", ko: "축소",
        ru: "Отдалить" }
    /// Fit-everything button.
    fit_all { en: "Fit all", pt: "Enquadrar tudo", es: "Ajustar todo",
        fr: "Tout afficher", de: "Alles einpassen", it: "Adatta tutto",
        ja: "全体を表示", zh: "适应全部", ko: "모두 맞추기", ru: "Показать всё" }
    /// Legend: grouped nodes.
    legend_grouped { en: "Grouped", pt: "Agrupadas", es: "Agrupadas", fr: "Regroupées",
        de: "Gruppiert", it: "Raggruppate", ja: "まとめ", zh: "已分组", ko: "묶음",
        ru: "Группы" }
    /// Legend: conflicting nodes.
    legend_conflict { en: "In conflict", pt: "Em conflito", es: "En conflicto",
        fr: "En conflit", de: "Im Konflikt", it: "In conflitto", ja: "競合中",
        zh: "存在冲突", ko: "충돌 중", ru: "Конфликт" }
    /// Legend: suggested links.
    legend_suggestion { en: "Suggestion", pt: "Sugestão", es: "Sugerencia",
        fr: "Suggestion", de: "Vorschlag", it: "Suggerimento", ja: "提案", zh: "建议",
        ko: "제안", ru: "Предложение" }
    /// Close the side card.
    close { en: "Close", pt: "Fechar", es: "Cerrar", fr: "Fermer", de: "Schließen",
        it: "Chiudi", ja: "閉じる", zh: "关闭", ko: "닫기", ru: "Закрыть" }
    /// Confirm a suggested link (accessible name).
    confirm_link_aria { en: "Confirm link", pt: "Confirmar ligação",
        es: "Confirmar vínculo", fr: "Confirmer le lien", de: "Verknüpfung bestätigen",
        it: "Conferma collegamento", ja: "リンクを確定", zh: "确认关联",
        ko: "연결 확인", ru: "Подтвердить ссылку" }
    /// Confirm button.
    confirm { en: "Confirm", pt: "Confirmar", es: "Confirmar", fr: "Confirmer",
        de: "Bestätigen", it: "Conferma", ja: "確定", zh: "确认", ko: "확인",
        ru: "Подтвердить" }
    /// Reject a suggested link (accessible name).
    reject_link_aria { en: "Reject link", pt: "Rejeitar ligação", es: "Rechazar vínculo",
        fr: "Rejeter le lien", de: "Verknüpfung ablehnen", it: "Rifiuta collegamento",
        ja: "リンクを却下", zh: "拒绝关联", ko: "연결 거부", ru: "Отклонить ссылку" }
    /// Reject button.
    reject { en: "Reject", pt: "Rejeitar", es: "Rechazar", fr: "Rejeter",
        de: "Ablehnen", it: "Rifiuta", ja: "却下", zh: "拒绝", ko: "거부",
        ru: "Отклонить" }
    /// Open a group's component in the map.
    open_all_in_map { en: "See all in Map", pt: "Ver todas no Mapa",
        es: "Ver todas en Mapa", fr: "Tout voir dans Carte", de: "Alle in Karte ansehen",
        it: "Vedi tutte in Mappa", ja: "マップですべて見る", zh: "在地图中查看全部",
        ko: "맵에서 모두 보기", ru: "Все в разделе «Карта»" }
    /// Open a node in the map.
    open_in_map { en: "Open in Map", pt: "Abrir no Mapa", es: "Abrir en Mapa",
        fr: "Ouvrir dans Carte", de: "In Karte öffnen", it: "Apri in Mappa",
        ja: "マップで開く", zh: "在地图中打开", ko: "맵에서 열기",
        ru: "Открыть в разделе «Карта»" }
    /// Open a decision in the decisions screen.
    open_in_decisions { en: "Open in Decisions", pt: "Abrir em Decisões",
        es: "Abrir en Decisiones", fr: "Ouvrir dans Décisions",
        de: "In Entscheidungen öffnen", it: "Apri in Decisioni", ja: "決定で開く",
        zh: "在决策中打开", ko: "결정에서 열기", ru: "Открыть в разделе «Решения»" }
    /// Card section: ties of the node.
    card_links { en: "Links", pt: "Ligações", es: "Vínculos", fr: "Liens",
        de: "Verknüpfungen", it: "Collegamenti", ja: "リンク", zh: "关联", ko: "연결",
        ru: "Ссылки" }
    /// Card section when the node has no confirmed ties.
    card_no_links { en: "No confirmed links", pt: "Sem ligações confirmadas",
        es: "Sin vínculos confirmados", fr: "Aucun lien confirmé",
        de: "Keine bestätigten Verknüpfungen", it: "Nessun collegamento confermato",
        ja: "確定したリンクはありません", zh: "没有已确认的关联",
        ko: "확인된 연결 없음", ru: "Нет подтверждённых ссылок" }
    /// Hint while the layout settles.
    hint_organizing { en: "Arranging the map…", pt: "Organizando o mapa…",
        es: "Organizando el mapa…", fr: "Organisation de la carte…",
        de: "Karte wird angeordnet …", it: "Sto organizzando la mappa…",
        ja: "マップを整理しています…", zh: "正在整理地图…", ko: "맵을 정리하는 중…",
        ru: "Раскладываю карту…" }
    /// Default hint about mouse gestures.
    hint_default { en: "Click to focus · drag to move · scroll to zoom",
        pt: "Clique para focar · arraste para mover · role para aproximar",
        es: "Haz clic para enfocar · arrastra para mover · desplaza para acercar",
        fr: "Cliquez pour cibler · glissez pour déplacer · faites défiler pour zoomer",
        de: "Klicken zum Fokussieren · Ziehen zum Bewegen · Scrollen zum Zoomen",
        it: "Clicca per mettere a fuoco · trascina per spostare · scorri per ingrandire",
        ja: "クリックでフォーカス · ドラッグで移動 · スクロールでズーム",
        zh: "点击聚焦 · 拖动移动 · 滚动缩放",
        ko: "클릭해서 집중 · 드래그해서 이동 · 스크롤해서 확대",
        ru: "Щёлкните, чтобы выделить · перетащите, чтобы сдвинуть · прокрутите, чтобы приблизить" }
    /// Weight line of a node without decisions.
    no_decisions { en: "no decisions", pt: "sem decisões", es: "sin decisiones",
        fr: "aucune décision", de: "keine Entscheidungen", it: "nessuna decisione",
        ja: "決定なし", zh: "没有决策", ko: "결정 없음", ru: "нет решений" }
    /// Tie: this node is part of the other (outgoing).
    tie_part_of_out { en: "part of", pt: "faz parte de", es: "forma parte de",
        fr: "fait partie de", de: "Teil von", it: "fa parte di", ja: "所属先",
        zh: "隶属于", ko: "소속", ru: "входит в" }
    /// Tie: the other node is part of this one.
    tie_part_of_in { en: "part of this component", pt: "parte deste componente",
        es: "parte de este componente", fr: "partie de ce composant",
        de: "Teil dieser Komponente", it: "parte di questo componente",
        ja: "このコンポーネントのパート", zh: "该组件的一部分", ko: "이 컴포넌트의 파트",
        ru: "часть этого компонента" }
    /// Tie: this decision affects the other (outgoing).
    tie_affects_out { en: "changes this component", pt: "muda este componente",
        es: "cambia este componente", fr: "modifie ce composant",
        de: "ändert diese Komponente", it: "modifica questo componente",
        ja: "このコンポーネントを変更", zh: "更改该组件", ko: "이 컴포넌트를 변경",
        ru: "меняет этот компонент" }
    /// Tie: a decision affects this node.
    tie_affects_in { en: "decision that changes this item",
        pt: "decisão que muda este item", es: "decisión que cambia este elemento",
        fr: "décision qui modifie cet élément", de: "Entscheidung, die dieses Element ändert",
        it: "decisione che modifica questo elemento", ja: "この項目を変更する決定",
        zh: "更改此项的决策", ko: "이 항목을 변경하는 결정",
        ru: "решение, меняющее этот элемент" }
    /// Tie: this decision uses the other technology (outgoing).
    tie_uses_out { en: "uses this technology", pt: "usa esta tecnologia",
        es: "usa esta tecnología", fr: "utilise cette technologie",
        de: "nutzt diese Technologie", it: "usa questa tecnologia",
        ja: "このテクノロジーを使用", zh: "使用该技术", ko: "이 기술을 사용",
        ru: "использует эту технологию" }
    /// Tie: a decision uses this technology.
    tie_uses_in { en: "decision that uses this technology",
        pt: "decisão que usa esta tecnologia", es: "decisión que usa esta tecnología",
        fr: "décision qui utilise cette technologie",
        de: "Entscheidung, die diese Technologie nutzt",
        it: "decisione che usa questa tecnologia", ja: "このテクノロジーを使用する決定",
        zh: "使用该技术的决策", ko: "이 기술을 사용하는 결정",
        ru: "решение, использующее эту технологию" }
    /// Tie: this rule applies to the other (outgoing).
    tie_applies_out { en: "applies to", pt: "vale para", es: "se aplica a",
        fr: "s’applique à", de: "gilt für", it: "vale per", ja: "適用先",
        zh: "适用于", ko: "적용 대상", ru: "применяется к" }
    /// Tie: a rule applies to this node.
    tie_applies_in { en: "rule that applies here", pt: "regra que vale aqui",
        es: "regla que se aplica aquí", fr: "règle qui s’applique ici",
        de: "Regel, die hier gilt", it: "regola che vale qui",
        ja: "ここに適用されるルール", zh: "适用于此处的规则", ko: "여기에 적용되는 규칙",
        ru: "правило, действующее здесь" }
    /// Tie: conflict.
    tie_conflict { en: "in conflict", pt: "em conflito", es: "en conflicto",
        fr: "en conflit", de: "im Konflikt", it: "in conflitto", ja: "競合中",
        zh: "存在冲突", ko: "충돌 중", ru: "конфликт" }
    /// Tie: this node depends on the other (outgoing).
    tie_relation_out { en: "depends on", pt: "depende de", es: "depende de",
        fr: "dépend de", de: "hängt ab von", it: "dipende da", ja: "依存先",
        zh: "依赖于", ko: "의존 대상", ru: "зависит от" }
    /// Tie: the other depends on this node.
    tie_relation_in { en: "depends on this", pt: "depende desta", es: "depende de esta",
        fr: "dépend de celui-ci", de: "hängt hiervon ab", it: "dipende da questo",
        ja: "これに依存", zh: "依赖于此项", ko: "이 항목에 의존", ru: "зависит от этого" }
    /// Tie: suggested link.
    tie_suggested { en: "suggested", pt: "sugerida", es: "sugerido", fr: "suggéré",
        de: "vorgeschlagen", it: "suggerito", ja: "提案", zh: "建议", ko: "제안됨",
        ru: "предложено" }
}

/// `n` and the noun in its singular or plural form.
fn counted(n: usize, singular: bool, one_form: &str, many_form: &str) -> String {
    format!("{n} {}", if singular { one_form } else { many_form })
}

/// `n` and the noun in the Russian form `n` asks for.
fn russian(n: usize, forms: [&str; 3]) -> String {
    format!("{n} {}", forms[russian_form(n)])
}

/// "1 decision" or "3 decisions".
pub fn decisions_count(n: usize) -> String {
    match super::current() {
        Language::English => counted(n, one(n), "decision", "decisions"),
        Language::Portuguese => counted(n, one(n), "decisão", "decisões"),
        Language::Spanish => counted(n, one(n), "decisión", "decisiones"),
        Language::French => counted(n, french_one(n), "décision", "décisions"),
        Language::German => counted(n, one(n), "Entscheidung", "Entscheidungen"),
        Language::Italian => counted(n, one(n), "decisione", "decisioni"),
        Language::Japanese => format!("決定{n}件"),
        Language::Chinese => format!("{n} 项决策"),
        Language::Korean => format!("결정 {n}개"),
        Language::Russian => russian(n, ["решение", "решения", "решений"]),
    }
}

/// "1 rule" or "3 rules".
pub fn rules_count(n: usize) -> String {
    match super::current() {
        Language::English => counted(n, one(n), "rule", "rules"),
        Language::Portuguese => counted(n, one(n), "regra", "regras"),
        Language::Spanish => counted(n, one(n), "regla", "reglas"),
        Language::French => counted(n, french_one(n), "règle", "règles"),
        Language::German => counted(n, one(n), "Regel", "Regeln"),
        Language::Italian => counted(n, one(n), "regola", "regole"),
        Language::Japanese => format!("ルール{n}件"),
        Language::Chinese => format!("{n} 条规则"),
        Language::Korean => format!("규칙 {n}개"),
        Language::Russian => russian(n, ["правило", "правила", "правил"]),
    }
}
