//! Copy for the map area. See [`crate::i18n`] for how entries are declared.
//!
//! Sentences that put some words in the strong face (names of decisions, a
//! file) mark them with [`BOLD`] around the words, so each language places
//! them where its own word order wants; [`spans`] turns the text into the
//! `(text, strong)` parts the screen lays out.

use super::{formats, strings, Language};

/// Marks the start and end of words shown in the strong face inside a
/// sentence. A control character no one types, so content can't forge it.
pub const BOLD: char = '\u{1f}';

/// Splits a sentence marked with [`BOLD`] into `(text, strong)` parts.
pub fn spans(text: &str) -> Vec<(String, bool)> {
    text.split(BOLD)
        .enumerate()
        .filter(|(_, part)| !part.is_empty())
        .map(|(index, part)| (part.to_owned(), index % 2 == 1))
        .collect()
}

/// Declares counted copy: one `fn name(count, args) -> String` per entry,
/// with the forms each language needs (two for most, three for Russian, one
/// for Japanese, Chinese and Korean). Every literal is a `format!` string
/// naming its arguments inline, `{count}` included.
macro_rules! plurals {
    ($(
        $(#[$meta:meta])*
        $name:ident ( $count:ident : usize $(, $arg:ident : $ty:ty)* $(,)? ) {
            en: [$en1:literal, $en2:literal], pt: [$pt1:literal, $pt2:literal],
            es: [$es1:literal, $es2:literal], fr: [$fr1:literal, $fr2:literal],
            de: [$de1:literal, $de2:literal], it: [$it1:literal, $it2:literal],
            ja: $ja:literal, zh: $zh:literal, ko: $ko:literal,
            ru: [$ru1:literal, $ru2:literal, $ru5:literal] $(,)?
        }
    )*) => {$(
        $(#[$meta])*
        pub fn $name($count: usize $(, $arg: $ty)*) -> String {
            use super::{french_one, one, russian_form};
            match super::current() {
                Language::English if one($count) => format!($en1),
                Language::English => format!($en2),
                Language::Portuguese if one($count) => format!($pt1),
                Language::Portuguese => format!($pt2),
                Language::Spanish if one($count) => format!($es1),
                Language::Spanish => format!($es2),
                Language::French if french_one($count) => format!($fr1),
                Language::French => format!($fr2),
                Language::German if one($count) => format!($de1),
                Language::German => format!($de2),
                Language::Italian if one($count) => format!($it1),
                Language::Italian => format!($it2),
                Language::Japanese => format!($ja),
                Language::Chinese => format!($zh),
                Language::Korean => format!($ko),
                Language::Russian => match russian_form($count) {
                    0 => format!($ru1),
                    1 => format!($ru2),
                    _ => format!($ru5),
                },
            }
        }
    )*};
}

// ---- placeholders ----------------------------------------------------------

strings! {
    /// Placeholder of the name field.
    name_placeholder { en: "E.g. storage-sqlite", pt: "Ex.: storage-sqlite",
        es: "Ej.: storage-sqlite", fr: "Ex. : storage-sqlite", de: "z. B. storage-sqlite",
        it: "Es.: storage-sqlite", ja: "例: storage-sqlite", zh: "例如:storage-sqlite",
        ko: "예: storage-sqlite", ru: "Напр.: storage-sqlite" }
    /// Placeholder of the path patterns field.
    patterns_placeholder { en: "E.g. crates/storage-sqlite/**, migrations/*.sql",
        pt: "Ex.: crates/storage-sqlite/**, migrations/*.sql",
        es: "Ej.: crates/storage-sqlite/**, migrations/*.sql",
        fr: "Ex. : crates/storage-sqlite/**, migrations/*.sql",
        de: "z. B. crates/storage-sqlite/**, migrations/*.sql",
        it: "Es.: crates/storage-sqlite/**, migrations/*.sql",
        ja: "例: crates/storage-sqlite/**, migrations/*.sql",
        zh: "例如:crates/storage-sqlite/**, migrations/*.sql",
        ko: "예: crates/storage-sqlite/**, migrations/*.sql",
        ru: "Напр.: crates/storage-sqlite/**, migrations/*.sql" }
    /// Placeholder of the aliases field.
    aliases_placeholder { en: "E.g. storage, database", pt: "Ex.: armazenamento, banco",
        es: "Ej.: almacenamiento, base de datos", fr: "Ex. : stockage, base de données",
        de: "z. B. Speicher, Datenbank", it: "Es.: archiviazione, database",
        ja: "例: ストレージ, データベース", zh: "例如:存储, 数据库",
        ko: "예: 스토리지, 데이터베이스", ru: "Напр.: хранилище, база данных" }
    /// Placeholder of the description field.
    description_placeholder { en: "One sentence about what it is",
        pt: "Uma frase sobre o que é", es: "Una frase sobre qué es",
        fr: "Une phrase sur ce que c’est", de: "Ein Satz dazu, was es ist",
        it: "Una frase su cos’è", ja: "何であるかを一文で",
        zh: "用一句话说明它是什么", ko: "무엇인지 한 문장으로",
        ru: "Одно предложение о том, что это" }
    /// Description of the component that holds the files at the root of a
    /// workspace; the database stores none, so every language shows its own.
    infra_root_description {
        en: "Root files: workspace manifest, toolchain and shared configuration",
        pt: "Arquivos da raiz: manifesto do workspace, toolchain e configuração comum",
        es: "Archivos de la raíz: manifiesto del workspace, toolchain y configuración común",
        fr: "Fichiers de la racine : manifeste du workspace, toolchain et configuration commune",
        de: "Dateien im Wurzelverzeichnis: Workspace-Manifest, Toolchain und gemeinsame Konfiguration",
        it: "File della radice: manifesto del workspace, toolchain e configurazione comune",
        ja: "ルートのファイル: ワークスペースのマニフェスト、ツールチェーン、共通設定",
        zh: "根目录文件:工作区清单、工具链和公共配置",
        ko: "루트 파일: 워크스페이스 매니페스트, 툴체인, 공통 설정",
        ru: "Файлы в корне: манифест рабочей области, инструментарий и общая конфигурация" }
    /// Placeholder of the file path field.
    path_placeholder { en: "E.g. crates/storage-sqlite/src/store.rs",
        pt: "Ex.: crates/storage-sqlite/src/store.rs",
        es: "Ej.: crates/storage-sqlite/src/store.rs",
        fr: "Ex. : crates/storage-sqlite/src/store.rs",
        de: "z. B. crates/storage-sqlite/src/store.rs",
        it: "Es.: crates/storage-sqlite/src/store.rs",
        ja: "例: crates/storage-sqlite/src/store.rs",
        zh: "例如:crates/storage-sqlite/src/store.rs",
        ko: "예: crates/storage-sqlite/src/store.rs",
        ru: "Напр.: crates/storage-sqlite/src/store.rs" }
    /// Placeholder of the picker's filter field.
    filter_placeholder { en: "Filter", pt: "Filtrar", es: "Filtrar", fr: "Filtrer",
        de: "Filtern", it: "Filtra", ja: "絞り込み", zh: "筛选", ko: "필터", ru: "Фильтр" }
}

// ---- notices and failures --------------------------------------------------

strings! {
    /// Notice: a suggested or graph link was confirmed.
    link_confirmed { en: "Link confirmed.", pt: "Vínculo confirmado.",
        es: "Vínculo confirmado.", fr: "Lien confirmé.", de: "Verknüpfung bestätigt.",
        it: "Collegamento confermato.", ja: "リンクを確定しました。",
        zh: "已确认关联。", ko: "연결을 확인했어요.", ru: "Ссылка подтверждена." }
    /// Notice: several suggested links were confirmed at once.
    links_confirmed_all { en: "Links confirmed.", pt: "Vínculos confirmados.",
        es: "Vínculos confirmados.", fr: "Liens confirmés.", de: "Verknüpfungen bestätigt.",
        it: "Collegamenti confermati.", ja: "リンクを確定しました。",
        zh: "已确认关联。", ko: "연결을 확인했어요.", ru: "Ссылки подтверждены." }
    /// Notice: a link was rejected.
    suggestion_rejected { en: "Suggestion rejected; it won't come back.",
        pt: "Sugestão rejeitada; ela não volta.", es: "Sugerencia rechazada; no volverá.",
        fr: "Suggestion rejetée ; elle ne reviendra pas.",
        de: "Vorschlag abgelehnt; er kommt nicht wieder.",
        it: "Suggerimento rifiutato; non tornerà.",
        ja: "提案を却下しました。再び表示されることはありません。",
        zh: "已拒绝该建议,不会再出现。",
        ko: "제안을 거부했어요. 다시 나타나지 않아요.",
        ru: "Предложение отклонено; оно не вернётся." }
    /// Notice: a link was recorded from the picker.
    link_recorded { en: "Link recorded.", pt: "Vínculo registrado.",
        es: "Vínculo registrado.", fr: "Lien enregistré.", de: "Verknüpfung erfasst.",
        it: "Collegamento registrato.", ja: "リンクを登録しました。",
        zh: "已记录关联。", ko: "연결을 등록했어요.", ru: "Ссылка записана." }
    /// Notice: a relation between decisions was recorded.
    relation_recorded { en: "Relation recorded.", pt: "Relação registrada.",
        es: "Relación registrada.", fr: "Relation enregistrée.", de: "Beziehung erfasst.",
        it: "Relazione registrata.", ja: "関係を登録しました。",
        zh: "已记录关系。", ko: "관계를 등록했어요.", ru: "Связь записана." }
    /// Notice: a relation between decisions was rejected.
    relation_rejected { en: "Relation rejected; it won't come back.",
        pt: "Relação rejeitada; ela não volta.", es: "Relación rechazada; no volverá.",
        fr: "Relation rejetée ; elle ne reviendra pas.",
        de: "Beziehung abgelehnt; sie kommt nicht wieder.",
        it: "Relazione rifiutata; non tornerà.",
        ja: "関係を却下しました。再び表示されることはありません。",
        zh: "已拒绝该关系,不会再出现。",
        ko: "관계를 거부했어요. 다시 나타나지 않아요.",
        ru: "Связь отклонена; она не вернётся." }
    /// Notice: a rule was created from a suggested context.
    context_rule_created { en: "Rule created from the decision.",
        pt: "Regra criada a partir da decisão.",
        es: "Regla creada a partir de la decisión.",
        fr: "Règle créée à partir de la décision.",
        de: "Regel aus der Entscheidung erstellt.",
        it: "Regola creata dalla decisione.",
        ja: "決定からルールを作成しました。", zh: "已根据该决策创建规则。",
        ko: "결정에서 규칙을 만들었어요.", ru: "Правило создано из решения." }
    /// Notice: an item was edited.
    item_updated { en: "Item updated.", pt: "Item atualizado.",
        es: "Elemento actualizado.", fr: "Élément mis à jour.", de: "Element aktualisiert.",
        it: "Elemento aggiornato.", ja: "項目を更新しました。", zh: "条目已更新。",
        ko: "항목을 업데이트했어요.", ru: "Элемент обновлён." }
    /// Notice: an item was created.
    item_created { en: "Item created. The suggestions were updated.",
        pt: "Item criado. As sugestões foram atualizadas.",
        es: "Elemento creado. Se actualizaron las sugerencias.",
        fr: "Élément créé. Les suggestions ont été mises à jour.",
        de: "Element erstellt. Die Vorschläge wurden aktualisiert.",
        it: "Elemento creato. I suggerimenti sono stati aggiornati.",
        ja: "項目を作成しました。提案を更新しました。",
        zh: "条目已创建,建议已更新。",
        ko: "항목을 만들었어요. 제안을 업데이트했어요.",
        ru: "Элемент создан. Предложения обновлены." }
    /// Notice: an item was retired.
    item_retired { en: "Item retired; its history remains.",
        pt: "Item aposentado; a história continua.",
        es: "Elemento retirado; el historial continúa.",
        fr: "Élément retiré ; l’historique reste.",
        de: "Element stillgelegt; der Verlauf bleibt erhalten.",
        it: "Elemento ritirato; la cronologia resta.",
        ja: "項目を廃止しました。履歴は残ります。",
        zh: "条目已停用,历史记录仍保留。",
        ko: "항목을 폐기했어요. 기록은 그대로 남아요.",
        ru: "Элемент списан; история сохраняется." }
    /// Notice: the components the workspace declares were created.
    workspace_components_created { en: "Workspace components created.",
        pt: "Componentes do workspace criados.",
        es: "Componentes del workspace creados.",
        fr: "Composants du workspace créés.",
        de: "Workspace-Komponenten erstellt.",
        it: "Componenti del workspace creati.",
        ja: "ワークスペースのコンポーネントを作成しました。",
        zh: "已创建工作区组件。", ko: "워크스페이스 컴포넌트를 만들었어요.",
        ru: "Компоненты рабочей области созданы." }
    /// Error: opening an item failed.
    cannot_open_item { en: "Couldn't open the item.", pt: "Não foi possível abrir o item.",
        es: "No se pudo abrir el elemento.", fr: "Impossible d’ouvrir l’élément.",
        de: "Das Element konnte nicht geöffnet werden.",
        it: "Impossibile aprire l’elemento.", ja: "項目を開けませんでした。",
        zh: "无法打开该条目。", ko: "항목을 열 수 없어요.",
        ru: "Не удалось открыть элемент." }
    /// Error: reading an item's history failed.
    cannot_read_history { en: "Couldn't read the history.",
        pt: "Não foi possível ler a história.", es: "No se pudo leer el historial.",
        fr: "Impossible de lire l’historique.",
        de: "Der Verlauf konnte nicht gelesen werden.",
        it: "Impossibile leggere la cronologia.", ja: "履歴を読み込めませんでした。",
        zh: "无法读取历史。", ko: "기록을 읽을 수 없어요.",
        ru: "Не удалось прочитать историю." }
    /// Error: reading the timeline failed.
    cannot_read_timeline { en: "Couldn't read the timeline.",
        pt: "Não foi possível ler a linha do tempo.",
        es: "No se pudo leer la línea de tiempo.",
        fr: "Impossible de lire la chronologie.",
        de: "Die Zeitleiste konnte nicht gelesen werden.",
        it: "Impossibile leggere la linea temporale.",
        ja: "タイムラインを読み込めませんでした。", zh: "无法读取时间线。",
        ko: "타임라인을 읽을 수 없어요.", ru: "Не удалось прочитать хронологию." }
    /// Error: listing decisions to link failed.
    cannot_list_decisions { en: "Couldn't list the decisions.",
        pt: "Não foi possível listar as decisões.",
        es: "No se pudieron listar las decisiones.",
        fr: "Impossible de lister les décisions.",
        de: "Die Entscheidungen konnten nicht aufgelistet werden.",
        it: "Impossibile elencare le decisioni.",
        ja: "決定の一覧を取得できませんでした。", zh: "无法列出决策。",
        ko: "결정 목록을 불러올 수 없어요.",
        ru: "Не удалось получить список решений." }
    /// Error: listing rules to link failed.
    cannot_list_rules { en: "Couldn't list the rules.",
        pt: "Não foi possível listar as regras.",
        es: "No se pudieron listar las reglas.", fr: "Impossible de lister les règles.",
        de: "Die Regeln konnten nicht aufgelistet werden.",
        it: "Impossibile elencare le regole.",
        ja: "ルールの一覧を取得できませんでした。", zh: "无法列出规则。",
        ko: "규칙 목록을 불러올 수 없어요.",
        ru: "Не удалось получить список правил." }
    /// Error: reading a file path in the file lens failed.
    cannot_read_path { en: "Couldn't read that path.",
        pt: "Não foi possível ler esse caminho.", es: "No se pudo leer esa ruta.",
        fr: "Impossible de lire ce chemin.",
        de: "Dieser Pfad konnte nicht gelesen werden.",
        it: "Impossibile leggere questo percorso.",
        ja: "そのパスを読み込めませんでした。", zh: "无法读取该路径。",
        ko: "그 경로를 읽을 수 없어요.", ru: "Не удалось прочитать этот путь." }
    /// Error: building the map from what the project declares failed.
    cannot_assemble { en: "Couldn't build the map from the project.",
        pt: "Não foi possível montar o mapa a partir do projeto.",
        es: "No se pudo crear el mapa a partir del proyecto.",
        fr: "Impossible de construire la carte à partir du projet.",
        de: "Die Karte konnte nicht aus dem Projekt erstellt werden.",
        it: "Impossibile creare la mappa dal progetto.",
        ja: "プロジェクトからマップを作成できませんでした。",
        zh: "无法根据项目生成地图。", ko: "프로젝트로 맵을 만들 수 없어요.",
        ru: "Не удалось собрать карту из проекта." }
    /// Error: refreshing the suggestions failed.
    cannot_refresh_suggestions { en: "Couldn't update the suggestions.",
        pt: "Não foi possível atualizar as sugestões.",
        es: "No se pudieron actualizar las sugerencias.",
        fr: "Impossible de mettre à jour les suggestions.",
        de: "Die Vorschläge konnten nicht aktualisiert werden.",
        it: "Impossibile aggiornare i suggerimenti.",
        ja: "提案を更新できませんでした。", zh: "无法更新建议。",
        ko: "제안을 업데이트할 수 없어요.", ru: "Не удалось обновить предложения." }
    /// Error: reading the map failed.
    cannot_read_map { en: "Couldn't read the map.", pt: "Não foi possível ler o mapa.",
        es: "No se pudo leer el mapa.", fr: "Impossible de lire la carte.",
        de: "Die Karte konnte nicht gelesen werden.",
        it: "Impossibile leggere la mappa.", ja: "マップを読み込めませんでした。",
        zh: "无法读取地图。", ko: "맵을 읽을 수 없어요.",
        ru: "Не удалось прочитать карту." }
    /// Error: reading the suggested links failed.
    cannot_read_suggestions { en: "Couldn't read the suggestions.",
        pt: "Não foi possível ler as sugestões.",
        es: "No se pudieron leer las sugerencias.",
        fr: "Impossible de lire les suggestions.",
        de: "Die Vorschläge konnten nicht gelesen werden.",
        it: "Impossibile leggere i suggerimenti.",
        ja: "提案を読み込めませんでした。", zh: "无法读取建议。",
        ko: "제안을 읽을 수 없어요.", ru: "Не удалось прочитать предложения." }
    /// Error: reading the graph failed.
    cannot_read_graph { en: "Couldn't read the graph.", pt: "Não foi possível ler o grafo.",
        es: "No se pudo leer el grafo.", fr: "Impossible de lire le graphe.",
        de: "Der Graph konnte nicht gelesen werden.",
        it: "Impossibile leggere il grafo.", ja: "グラフを読み込めませんでした。",
        zh: "无法读取图谱。", ko: "그래프를 읽을 수 없어요.",
        ru: "Не удалось прочитать граф." }
    /// Error: reading the suggested relations failed.
    cannot_read_relation_suggestions { en: "Couldn't read the suggested relations.",
        pt: "Não foi possível ler as relações sugeridas.",
        es: "No se pudieron leer las relaciones sugeridas.",
        fr: "Impossible de lire les relations suggérées.",
        de: "Die vorgeschlagenen Beziehungen konnten nicht gelesen werden.",
        it: "Impossibile leggere le relazioni suggerite.",
        ja: "提案された関係を読み込めませんでした。", zh: "无法读取建议的关系。",
        ko: "제안된 관계를 읽을 수 없어요.",
        ru: "Не удалось прочитать предложенные связи." }
    /// Error: reading the suggested context failed.
    cannot_read_context_suggestions { en: "Couldn't read the suggested context.",
        pt: "Não foi possível ler o contexto sugerido.",
        es: "No se pudo leer el contexto sugerido.",
        fr: "Impossible de lire le contexte suggéré.",
        de: "Der vorgeschlagene Kontext konnte nicht gelesen werden.",
        it: "Impossibile leggere il contesto suggerito.",
        ja: "提案されたコンテキストを読み込めませんでした。",
        zh: "无法读取建议的上下文。", ko: "제안된 컨텍스트를 읽을 수 없어요.",
        ru: "Не удалось прочитать предложенный контекст." }
    /// Error: reading the files the decisions touched failed.
    cannot_read_decision_files { en: "Couldn't read the decisions' files.",
        pt: "Não foi possível ler os arquivos das decisões.",
        es: "No se pudieron leer los archivos de las decisiones.",
        fr: "Impossible de lire les fichiers des décisions.",
        de: "Die Dateien der Entscheidungen konnten nicht gelesen werden.",
        it: "Impossibile leggere i file delle decisioni.",
        ja: "決定のファイルを読み込めませんでした。",
        zh: "无法读取决策涉及的文件。", ko: "결정의 파일을 읽을 수 없어요.",
        ru: "Не удалось прочитать файлы решений." }
    /// Error: a suggested context changed under the person.
    suggestion_changed { en: "The suggestion changed. The map was updated.",
        pt: "A sugestão mudou. O mapa foi atualizado.",
        es: "La sugerencia cambió. Se actualizó el mapa.",
        fr: "La suggestion a changé. La carte a été mise à jour.",
        de: "Der Vorschlag hat sich geändert. Die Karte wurde aktualisiert.",
        it: "Il suggerimento è cambiato. La mappa è stata aggiornata.",
        ja: "提案が変更されました。マップを更新しました。",
        zh: "该建议已变更,地图已更新。",
        ko: "제안이 바뀌었어요. 맵을 업데이트했어요.",
        ru: "Предложение изменилось. Карта обновлена." }
    /// Error: the rule from a suggested context could not be created.
    rule_not_created { en: "The rule couldn't be created; it may already exist.",
        pt: "A regra não pôde ser criada; ela pode já existir.",
        es: "No se pudo crear la regla; puede que ya exista.",
        fr: "La règle n’a pas pu être créée ; elle existe peut-être déjà.",
        de: "Die Regel konnte nicht erstellt werden; sie existiert möglicherweise schon.",
        it: "Non è stato possibile creare la regola; potrebbe già esistere.",
        ja: "ルールを作成できませんでした。すでに存在する可能性があります。",
        zh: "无法创建该规则,它可能已存在。",
        ko: "규칙을 만들 수 없어요. 이미 있을 수 있어요.",
        ru: "Не удалось создать правило; возможно, оно уже есть." }
    /// Error: saving the rule failed.
    cannot_save_rule { en: "Couldn't save the rule.",
        pt: "Não foi possível salvar a regra.", es: "No se pudo guardar la regla.",
        fr: "Impossible d’enregistrer la règle.",
        de: "Die Regel konnte nicht gespeichert werden.",
        it: "Impossibile salvare la regola.", ja: "ルールを保存できませんでした。",
        zh: "无法保存规则。", ko: "규칙을 저장할 수 없어요.",
        ru: "Не удалось сохранить правило." }
    /// Error: a relation between decisions no longer fits.
    relation_no_longer_fits { en: "That relation no longer fits (a cycle, or already recorded).",
        pt: "Essa relação não cabe mais (ciclo ou já registrada).",
        es: "Esa relación ya no cabe (ciclo o ya registrada).",
        fr: "Cette relation n’est plus possible (cycle ou déjà enregistrée).",
        de: "Diese Beziehung ist nicht mehr möglich (Zyklus oder bereits erfasst).",
        it: "Questa relazione non è più possibile (ciclo o già registrata).",
        ja: "この関係はもう登録できません(循環、または登録済み)。",
        zh: "该关系已无法添加(形成循环或已存在)。",
        ko: "이 관계는 더 이상 만들 수 없어요(순환이거나 이미 등록됨).",
        ru: "Эта связь больше невозможна (цикл или уже записана)." }
    /// Error: one of the decisions of a relation changed.
    decision_changed { en: "One of the decisions changed. The map was updated.",
        pt: "Uma das decisões mudou. O mapa foi atualizado.",
        es: "Una de las decisiones cambió. Se actualizó el mapa.",
        fr: "L’une des décisions a changé. La carte a été mise à jour.",
        de: "Eine der Entscheidungen hat sich geändert. Die Karte wurde aktualisiert.",
        it: "Una delle decisioni è cambiata. La mappa è stata aggiornata.",
        ja: "決定のひとつが変更されました。マップを更新しました。",
        zh: "其中一项决策已变更,地图已更新。",
        ko: "결정 중 하나가 바뀌었어요. 맵을 업데이트했어요.",
        ru: "Одно из решений изменилось. Карта обновлена." }
    /// Error: recording a relation failed.
    cannot_record_relation { en: "Couldn't record the relation.",
        pt: "Não foi possível registrar a relação.",
        es: "No se pudo registrar la relación.",
        fr: "Impossible d’enregistrer la relation.",
        de: "Die Beziehung konnte nicht erfasst werden.",
        it: "Impossibile registrare la relazione.", ja: "関係を登録できませんでした。",
        zh: "无法记录该关系。", ko: "관계를 등록할 수 없어요.",
        ru: "Не удалось записать связь." }
    /// Error: a map item with that name or alias exists.
    duplicate_name { en: "An item of this type with that name or alias already exists.",
        pt: "Já existe um item desse tipo com esse nome ou apelido.",
        es: "Ya existe un elemento de este tipo con ese nombre o alias.",
        fr: "Un élément de ce type avec ce nom ou cet alias existe déjà.",
        de: "Ein Element dieses Typs mit diesem Namen oder Alias gibt es schon.",
        it: "Esiste già un elemento di questo tipo con quel nome o alias.",
        ja: "この種類で同じ名前または別名の項目がすでにあります。",
        zh: "已存在同类型且名称或别名相同的条目。",
        ko: "같은 유형에 같은 이름이나 별칭의 항목이 이미 있어요.",
        ru: "Элемент этого типа с таким названием или псевдонимом уже есть." }
    /// Error: the link already exists.
    duplicate_edge { en: "That link already exists.", pt: "Esse vínculo já existe.",
        es: "Ese vínculo ya existe.", fr: "Ce lien existe déjà.",
        de: "Diese Verknüpfung gibt es schon.", it: "Questo collegamento esiste già.",
        ja: "そのリンクはすでにあります。", zh: "该关联已存在。",
        ko: "이미 있는 연결이에요.", ru: "Такая ссылка уже есть." }
    /// Error: the name has no letter or digit.
    empty_name { en: "Give it a name with at least one letter or digit.",
        pt: "Dê um nome com ao menos uma letra ou dígito.",
        es: "Pon un nombre con al menos una letra o un dígito.",
        fr: "Donnez un nom avec au moins une lettre ou un chiffre.",
        de: "Gib einen Namen mit mindestens einem Buchstaben oder einer Ziffer ein.",
        it: "Dai un nome con almeno una lettera o una cifra.",
        ja: "文字か数字を1つ以上含む名前を付けてください。",
        zh: "请起一个至少包含一个字母或数字的名称。",
        ko: "글자나 숫자가 하나 이상 들어간 이름을 지어 주세요.",
        ru: "Дайте название хотя бы с одной буквой или цифрой." }
    /// Error: the name is over 80 characters.
    name_too_long { en: "The name is over 80 characters.",
        pt: "O nome passa de 80 caracteres.", es: "El nombre supera los 80 caracteres.",
        fr: "Le nom dépasse 80 caractères.", de: "Der Name ist länger als 80 Zeichen.",
        it: "Il nome supera gli 80 caratteri.", ja: "名前が80文字を超えています。",
        zh: "名称超过 80 个字符。", ko: "이름이 80자를 넘어요.",
        ru: "Название длиннее 80 символов." }
    /// Error: the description is over 500 characters.
    description_too_long { en: "The description is over 500 characters.",
        pt: "A descrição passa de 500 caracteres.",
        es: "La descripción supera los 500 caracteres.",
        fr: "La description dépasse 500 caractères.",
        de: "Die Beschreibung ist länger als 500 Zeichen.",
        it: "La descrizione supera i 500 caratteri.",
        ja: "説明が500文字を超えています。", zh: "描述超过 500 个字符。",
        ko: "설명이 500자를 넘어요.", ru: "Описание длиннее 500 символов." }
    /// Error: a path pattern is not relative to the project.
    invalid_pattern { en: "Use paths relative to the project, like crates/app/**, with no .. or \
             drive letter.",
        pt: "Use caminhos relativos ao projeto, como crates/app/**, sem .. nem letra de disco.",
        es: "Usa rutas relativas al proyecto, como crates/app/**, sin .. ni letra de unidad.",
        fr: "Utilisez des chemins relatifs au projet, comme crates/app/**, sans .. ni lettre de \
             lecteur.",
        de: "Verwende projektrelative Pfade wie crates/app/**, ohne .. und ohne \
             Laufwerksbuchstaben.",
        it: "Usa percorsi relativi al progetto, come crates/app/**, senza .. né lettera di unità.",
        ja: "crates/app/** のようにプロジェクトからの相対パスを使い、.. やドライブ文字は含めないでください。",
        zh: "请使用相对于项目的路径,如 crates/app/**,不要包含 .. 或盘符。",
        ko: "crates/app/**처럼 프로젝트 기준 상대 경로를 쓰고, ..이나 드라이브 문자는 넣지 마세요.",
        ru: "Используйте пути относительно проекта, например crates/app/**, без .. и буквы диска." }
    /// Error: the link would create a cycle of components.
    cycle { en: "That link would create a cycle of components.",
        pt: "Esse vínculo criaria um ciclo de componentes.",
        es: "Ese vínculo crearía un ciclo de componentes.",
        fr: "Ce lien créerait un cycle de composants.",
        de: "Diese Verknüpfung würde einen Komponentenzyklus erzeugen.",
        it: "Questo collegamento creerebbe un ciclo di componenti.",
        ja: "このリンクではコンポーネントの循環ができてしまいます。",
        zh: "该关联会造成组件循环。", ko: "이 연결은 컴포넌트 순환을 만들어요.",
        ru: "Эта ссылка создала бы цикл компонентов." }
    /// Error: a component can't be part of itself.
    self_reference { en: "A component can't be part of itself.",
        pt: "Um componente não pode fazer parte de si mesmo.",
        es: "Un componente no puede formar parte de sí mismo.",
        fr: "Un composant ne peut pas faire partie de lui-même.",
        de: "Eine Komponente kann nicht Teil ihrer selbst sein.",
        it: "Un componente non può far parte di se stesso.",
        ja: "コンポーネント自身を所属先にすることはできません。",
        zh: "组件不能隶属于它自己。", ko: "컴포넌트는 자기 자신에 속할 수 없어요.",
        ru: "Компонент не может входить в самого себя." }
    /// Error: the component already has a parent.
    second_parent { en: "That component is already part of another. Undo the old link first.",
        pt: "Esse componente já faz parte de outro. Desfaça o vínculo antigo primeiro.",
        es: "Ese componente ya forma parte de otro. Deshaz primero el vínculo anterior.",
        fr: "Ce composant fait déjà partie d’un autre. Défaites d’abord l’ancien lien.",
        de: "Diese Komponente ist bereits Teil einer anderen. Löse zuerst die alte Verknüpfung.",
        it: "Questo componente fa già parte di un altro. Sciogli prima il vecchio collegamento.",
        ja: "このコンポーネントはすでに別のコンポーネントに属しています。先に古いリンクを解除してください。",
        zh: "该组件已隶属于另一个组件。请先解除旧的关联。",
        ko: "이 컴포넌트는 이미 다른 컴포넌트에 속해 있어요. 먼저 이전 연결을 해제해 주세요.",
        ru: "Этот компонент уже входит в другой. Сначала разорвите старую связь." }
    /// Error: the item changed while it was being edited.
    edit_conflict { en: "The item changed while you were editing. The map was updated.",
        pt: "O item mudou enquanto você editava. O mapa foi atualizado.",
        es: "El elemento cambió mientras lo editabas. Se actualizó el mapa.",
        fr: "L’élément a changé pendant que vous le modifiiez. La carte a été mise à jour.",
        de: "Das Element hat sich geändert, während du es bearbeitet hast. Die Karte wurde \
             aktualisiert.",
        it: "L’elemento è cambiato mentre lo modificavi. La mappa è stata aggiornata.",
        ja: "編集中に項目が変更されました。マップを更新しました。",
        zh: "你编辑时该条目已发生变化,地图已更新。",
        ko: "편집하는 동안 항목이 바뀌었어요. 맵을 업데이트했어요.",
        ru: "Элемент изменился, пока вы его редактировали. Карта обновлена." }
    /// Error: too many patterns or aliases.
    invalid_request { en: "Use up to 20 patterns and 20 aliases per item.",
        pt: "Use até 20 padrões e 20 apelidos por item.",
        es: "Usa hasta 20 patrones y 20 alias por elemento.",
        fr: "Utilisez jusqu’à 20 motifs et 20 alias par élément.",
        de: "Verwende höchstens 20 Muster und 20 Aliasse pro Element.",
        it: "Usa fino a 20 pattern e 20 alias per elemento.",
        ja: "1つの項目に使えるパターンと別名は、それぞれ20個までです。",
        zh: "每个条目最多使用 20 个模式和 20 个别名。",
        ko: "항목당 패턴과 별칭은 각각 20개까지 쓸 수 있어요.",
        ru: "Используйте не более 20 шаблонов и 20 псевдонимов на элемент." }
    /// Error: saving a change to the map failed.
    cannot_save_change { en: "Couldn't save the change to the map.",
        pt: "Não foi possível salvar a mudança no mapa.",
        es: "No se pudo guardar el cambio en el mapa.",
        fr: "Impossible d’enregistrer la modification de la carte.",
        de: "Die Änderung an der Karte konnte nicht gespeichert werden.",
        it: "Impossibile salvare la modifica alla mappa.",
        ja: "マップの変更を保存できませんでした。", zh: "无法保存对地图的更改。",
        ko: "맵 변경 사항을 저장할 수 없어요.",
        ru: "Не удалось сохранить изменение на карте." }
    /// Empty state title: the map could not load.
    map_load_failed_title { en: "Couldn't load the map",
        pt: "Não foi possível carregar o mapa", es: "No se pudo cargar el mapa",
        fr: "Impossible de charger la carte",
        de: "Die Karte konnte nicht geladen werden",
        it: "Impossibile caricare la mappa", ja: "マップを読み込めませんでした",
        zh: "无法加载地图", ko: "맵을 불러올 수 없어요",
        ru: "Не удалось загрузить карту" }
    /// Empty state hint: the map could not load.
    map_load_failed_hint { en: "Try again; your data wasn't changed.",
        pt: "Tente de novo; seus dados não foram alterados.",
        es: "Inténtalo de nuevo; tus datos no se modificaron.",
        fr: "Réessayez ; vos données n’ont pas été modifiées.",
        de: "Versuche es erneut; deine Daten wurden nicht verändert.",
        it: "Riprova; i tuoi dati non sono stati modificati.",
        ja: "もう一度お試しください。データは変更されていません。",
        zh: "请重试;你的数据没有被更改。",
        ko: "다시 시도해 보세요. 데이터는 바뀌지 않았어요.",
        ru: "Попробуйте ещё раз; ваши данные не изменены." }
}

plurals! {
    /// Notice: the map was assembled from what the workspace declares.
    assembled_from_workspace(count: usize) {
        en: ["Map built from the workspace: {count} component.",
            "Map built from the workspace: {count} components."],
        pt: ["Mapa montado a partir do workspace: {count} componente.",
            "Mapa montado a partir do workspace: {count} componentes."],
        es: ["Mapa creado a partir del workspace: {count} componente.",
            "Mapa creado a partir del workspace: {count} componentes."],
        fr: ["Carte construite à partir du workspace : {count} composant.",
            "Carte construite à partir du workspace : {count} composants."],
        de: ["Karte aus dem Workspace erstellt: {count} Komponente.",
            "Karte aus dem Workspace erstellt: {count} Komponenten."],
        it: ["Mappa creata dal workspace: {count} componente.",
            "Mappa creata dal workspace: {count} componenti."],
        ja: "ワークスペースからマップを作成しました: コンポーネント{count}件。",
        zh: "已根据工作区生成地图:{count} 个组件。",
        ko: "워크스페이스로 맵을 만들었어요: 컴포넌트 {count}개.",
        ru: ["Карта собрана из рабочей области: {count} компонент.",
            "Карта собрана из рабочей области: {count} компонента.",
            "Карта собрана из рабочей области: {count} компонентов."]
    }
}

// ---- index, buttons and shared labels --------------------------------------

strings! {
    /// Title of the index panel.
    map_title { en: "Map", pt: "Mapa", es: "Mapa", fr: "Carte", de: "Karte", it: "Mappa",
        ja: "マップ", zh: "地图", ko: "맵", ru: "Карта" }
    /// Label of the button that opens the form for a new item.
    new_item { en: "New component or technology", pt: "Novo componente ou tecnologia",
        es: "Nuevo componente o tecnología", fr: "Nouveau composant ou technologie",
        de: "Neue Komponente oder Technologie", it: "Nuovo componente o tecnologia",
        ja: "新しいコンポーネントまたはテクノロジー", zh: "新建组件或技术",
        ko: "새 컴포넌트 또는 기술", ru: "Новый компонент или технология" }
    /// Index footer while the map loads.
    loading { en: "Loading…", pt: "Carregando…", es: "Cargando…", fr: "Chargement…",
        de: "Wird geladen…", it: "Caricamento…", ja: "読み込み中…", zh: "加载中…",
        ko: "불러오는 중…", ru: "Загрузка…" }
    /// Index: what a kind with no entities says (components).
    index_empty_components { en: "None yet. Create one or see the suggestions.",
        pt: "Nenhum ainda. Crie um ou veja as sugestões.",
        es: "Ninguno aún. Crea uno o mira las sugerencias.",
        fr: "Aucun pour l’instant. Créez-en un ou consultez les suggestions.",
        de: "Noch keine. Erstelle eine oder sieh dir die Vorschläge an.",
        it: "Nessuno per ora. Creane uno o guarda i suggerimenti.",
        ja: "まだありません。作成するか、提案をご覧ください。",
        zh: "暂无。新建一个,或查看建议。",
        ko: "아직 없어요. 만들거나 제안을 확인해 보세요.",
        ru: "Пока нет. Создайте или посмотрите предложения." }
    /// Index: what a kind with no entities says (technologies).
    index_empty_technologies { en: "None yet.", pt: "Nenhuma ainda.",
        es: "Ninguna aún.", fr: "Aucune pour l’instant.", de: "Noch keine.",
        it: "Nessuna per ora.", ja: "まだありません。", zh: "暂无。",
        ko: "아직 없어요.", ru: "Пока нет." }
    /// Index view: the overview of the map.
    view_overview { en: "Overview", pt: "Visão geral", es: "Visión general",
        fr: "Vue d’ensemble", de: "Übersicht", it: "Panoramica", ja: "概要", zh: "概览",
        ko: "개요", ru: "Обзор" }
    /// Index view and page title: what holds for one file.
    view_file_lens { en: "File lens", pt: "Lente de arquivo", es: "Lente de archivo",
        fr: "Vue par fichier", de: "Dateiansicht", it: "Vista per file",
        ja: "ファイルレンズ", zh: "文件视角", ko: "파일 렌즈", ru: "Обзор файла" }
    /// Index view, page title and section: the project's timeline.
    view_timeline { en: "Timeline", pt: "Linha do tempo", es: "Línea de tiempo",
        fr: "Chronologie", de: "Zeitleiste", it: "Linea temporale", ja: "タイムライン",
        zh: "时间线", ko: "타임라인", ru: "Хронология" }
    /// Button: confirm a suggestion.
    confirm { en: "Confirm", pt: "Confirmar", es: "Confirmar", fr: "Confirmer",
        de: "Bestätigen", it: "Conferma", ja: "確定", zh: "确认", ko: "확인",
        ru: "Подтвердить" }
    /// Button: reject a suggestion.
    reject { en: "Reject", pt: "Rejeitar", es: "Rechazar", fr: "Rejeter",
        de: "Ablehnen", it: "Rifiuta", ja: "却下", zh: "拒绝", ko: "거부", ru: "Отклонить" }
    /// Button: cancel.
    cancel { en: "Cancel", pt: "Cancelar", es: "Cancelar", fr: "Annuler",
        de: "Abbrechen", it: "Annulla", ja: "キャンセル", zh: "取消", ko: "취소",
        ru: "Отмена" }
    /// Button: close the picker.
    close { en: "Close", pt: "Fechar", es: "Cerrar", fr: "Fermer", de: "Schließen",
        it: "Chiudi", ja: "閉じる", zh: "关闭", ko: "닫기", ru: "Закрыть" }
    /// Button: try the failed read again (error banner).
    try_again { en: "Try again", pt: "Tentar de novo", es: "Reintentar",
        fr: "Réessayer", de: "Erneut versuchen", it: "Riprova", ja: "再試行",
        zh: "重试", ko: "다시 시도", ru: "Повторить" }
    /// Button: try again, inside a suggestion that could not be read.
    try_again_inline { en: "Try again", pt: "Tentar novamente", es: "Reintentar",
        fr: "Réessayer", de: "Erneut versuchen", it: "Riprova", ja: "再試行",
        zh: "重试", ko: "다시 시도", ru: "Повторить" }
    /// Button: create the suggested item.
    create { en: "Create", pt: "Criar", es: "Crear", fr: "Créer", de: "Erstellen",
        it: "Crea", ja: "作成", zh: "创建", ko: "만들기", ru: "Создать" }
    /// Button: save the form.
    save { en: "Save", pt: "Salvar", es: "Guardar", fr: "Enregistrer",
        de: "Speichern", it: "Salva", ja: "保存", zh: "保存", ko: "저장",
        ru: "Сохранить" }
    /// Tooltip of the rows that open a decision.
    open_in_decisions { en: "Open in Decisions", pt: "Abrir em Decisões",
        es: "Abrir en Decisiones", fr: "Ouvrir dans Décisions",
        de: "In Entscheidungen öffnen", it: "Apri in Decisioni", ja: "決定で開く",
        zh: "在“决策”中打开", ko: "결정에서 열기", ru: "Открыть в «Решениях»" }
    /// Name of a kind of item: a component.
    kind_component { en: "Component", pt: "Componente", es: "Componente",
        fr: "Composant", de: "Komponente", it: "Componente", ja: "コンポーネント",
        zh: "组件", ko: "컴포넌트", ru: "Компонент" }
    /// Name of a kind of item: a technology.
    kind_technology { en: "Technology", pt: "Tecnologia", es: "Tecnología",
        fr: "Technologie", de: "Technologie", it: "Tecnologia", ja: "テクノロジー",
        zh: "技术", ko: "기술", ru: "Технология" }
    /// Heading of the components (index and overview).
    kind_components { en: "Components", pt: "Componentes", es: "Componentes",
        fr: "Composants", de: "Komponenten", it: "Componenti", ja: "コンポーネント",
        zh: "组件", ko: "컴포넌트", ru: "Компоненты" }
    /// Heading of the technologies (index and overview).
    kind_technologies { en: "Technologies", pt: "Tecnologias", es: "Tecnologías",
        fr: "Technologies", de: "Technologien", it: "Tecnologie", ja: "テクノロジー",
        zh: "技术", ko: "기술", ru: "Технологии" }
    /// A kind of rule: an assumption.
    claim_assumption { en: "Assumption", pt: "Premissa", es: "Supuesto",
        fr: "Hypothèse", de: "Annahme", it: "Presupposto", ja: "前提", zh: "前提",
        ko: "전제", ru: "Допущение" }
    /// A kind of rule: a constraint.
    claim_constraint { en: "Constraint", pt: "Restrição", es: "Restricción",
        fr: "Contrainte", de: "Einschränkung", it: "Vincolo", ja: "制約", zh: "约束",
        ko: "제약", ru: "Ограничение" }
    /// A kind of rule: a goal.
    claim_goal { en: "Goal", pt: "Objetivo", es: "Objetivo", fr: "Objectif",
        de: "Ziel", it: "Obiettivo", ja: "目標", zh: "目标", ko: "목표", ru: "Цель" }
    /// A kind of rule: a convention.
    claim_convention { en: "Convention", pt: "Convenção", es: "Convención",
        fr: "Convention", de: "Konvention", it: "Convenzione", ja: "規約", zh: "约定",
        ko: "규약", ru: "Соглашение" }
    /// A kind of rule: a plain rule.
    claim_rule { en: "Rule", pt: "Regra", es: "Regla", fr: "Règle", de: "Regel",
        it: "Regola", ja: "ルール", zh: "规则", ko: "규칙", ru: "Правило" }
    /// Name of the Cargo workspace manifest, as a source of components.
    workspace_cargo { en: "Cargo workspace", pt: "workspace Cargo",
        es: "Cargo workspace", fr: "Cargo workspace", de: "Cargo workspace",
        it: "Cargo workspace", ja: "Cargo workspace", zh: "Cargo workspace",
        ko: "Cargo workspace", ru: "Cargo workspace" }
    /// Name of the npm workspaces setting, as a source of components.
    workspace_npm { en: "npm workspaces", pt: "workspaces do npm",
        es: "npm workspaces", fr: "npm workspaces", de: "npm workspaces",
        it: "npm workspaces", ja: "npm workspaces", zh: "npm workspaces",
        ko: "npm workspaces", ru: "npm workspaces" }
    /// Name of the pnpm workspace file, as a source of components.
    workspace_pnpm { en: "pnpm workspace", pt: "workspace pnpm",
        es: "pnpm workspace", fr: "pnpm workspace", de: "pnpm workspace",
        it: "pnpm workspace", ja: "pnpm workspace", zh: "pnpm workspace",
        ko: "pnpm workspace", ru: "pnpm workspace" }
}

/// The name of a kind of rule (`assumption`, `constraint`, `goal`,
/// `convention`; anything else reads as a plain rule).
pub fn claim_label(kind: &str) -> &'static str {
    match kind {
        "assumption" => claim_assumption(),
        "constraint" => claim_constraint(),
        "goal" => claim_goal(),
        "convention" => claim_convention(),
        _ => claim_rule(),
    }
}

formats! {
    /// Description of a continuous integration component; `pattern` is where
    /// its pipelines live.
    infra_ci_description(pattern: &str) {
        en: "Continuous integration and release ({pattern})",
        pt: "Integração contínua e publicação ({pattern})",
        es: "Integración continua y publicación ({pattern})",
        fr: "Intégration continue et publication ({pattern})",
        de: "Continuous Integration und Veröffentlichung ({pattern})",
        it: "Integrazione continua e pubblicazione ({pattern})",
        ja: "継続的インテグレーションとリリース ({pattern})",
        zh: "持续集成与发布({pattern})",
        ko: "지속적 통합 및 배포 ({pattern})",
        ru: "Непрерывная интеграция и выпуск ({pattern})" }
    /// Under a suggested tie: a word that may negate it sits near the excerpt
    /// (`trigger`, as written in `quote`), so check the polarity before
    /// confirming.
    link_polarity_alert(trigger: &str, quote: &str) {
        en: "Check the polarity: “{trigger}” in “{quote}” may negate this link.",
        pt: "Confira a polaridade: “{trigger}” em “{quote}” pode negar este vínculo.",
        es: "Revisa la polaridad: “{trigger}” en “{quote}” puede negar este vínculo.",
        fr: "Vérifiez la polarité : « {trigger} » dans « {quote} » peut nier ce lien.",
        de: "Polarität prüfen: „{trigger}“ in „{quote}“ kann diese Verknüpfung verneinen.",
        it: "Verifica la polarità: “{trigger}” in “{quote}” può negare questo collegamento.",
        ja: "極性を確認: 「{quote}」の中の「{trigger}」がこの関連を否定している可能性があります。",
        zh: "请确认极性:“{quote}”中的“{trigger}”可能否定了这条关联。",
        ko: "극성 확인: “{quote}” 안의 “{trigger}”이(가) 이 연결을 부정할 수 있습니다.",
        ru: "Проверьте полярность: «{trigger}» в «{quote}» может отрицать эту связь." }
    /// Button: show the next page of a long list.
    show_more(step: usize) { en: "Show {step} more", pt: "Mostrar mais {step}",
        es: "Mostrar {step} más", fr: "Afficher {step} de plus",
        de: "{step} weitere anzeigen", it: "Mostra altri {step}",
        ja: "さらに{step}件を表示", zh: "再显示 {step} 项", ko: "{step}개 더 보기",
        ru: "Показать ещё {step}" }
    /// Button: show everything that is left of a long list.
    show_all(remaining: usize) { en: "Show all ({remaining})",
        pt: "Mostrar todas ({remaining})", es: "Mostrar todas ({remaining})",
        fr: "Tout afficher ({remaining})", de: "Alle anzeigen ({remaining})",
        it: "Mostra tutte ({remaining})", ja: "すべて表示 ({remaining})",
        zh: "显示全部({remaining})", ko: "모두 보기 ({remaining})",
        ru: "Показать все ({remaining})" }
    /// Index footer: the date the map is read as of.
    status_as_of(date: &str) { en: "Status as of {date}", pt: "Situação em {date}",
        es: "Situación al {date}", fr: "Situation au {date}", de: "Stand: {date}",
        it: "Situazione al {date}", ja: "{date}時点の状況", zh: "截至 {date} 的状态",
        ko: "{date} 기준 상태", ru: "Состояние на {date}" }
}

// ---- suggestions page ------------------------------------------------------

strings! {
    /// Title of the suggestions page and its empty state.
    suggestions_title { en: "Suggestions", pt: "Sugestões", es: "Sugerencias",
        fr: "Suggestions", de: "Vorschläge", it: "Suggerimenti", ja: "提案",
        zh: "建议", ko: "제안", ru: "Предложения" }
    /// Empty state title of the suggestions page.
    nothing_to_review { en: "Nothing to review", pt: "Nada para revisar",
        es: "Nada que revisar", fr: "Rien à revoir", de: "Nichts zu prüfen",
        it: "Niente da rivedere", ja: "確認するものはありません",
        zh: "没有需要审阅的内容", ko: "검토할 항목이 없어요", ru: "Нечего проверять" }
    /// Empty state hint of the suggestions page.
    suggestions_empty_hint {
        en: "Suggestions appear when confirmed decisions touch files or add dependencies. \
             Create components with path patterns to link them to decisions.",
        pt: "Sugestões aparecem quando decisões confirmadas tocam arquivos ou adicionam \
             dependências. Crie componentes com padrões de caminho para ligá-los às decisões.",
        es: "Las sugerencias aparecen cuando decisiones confirmadas tocan archivos o añaden \
             dependencias. Crea componentes con patrones de ruta para vincularlos a las decisione\
             s.",
        fr: "Les suggestions apparaissent quand des décisions confirmées touchent des fichiers \
             ou ajoutent des dépendances. Créez des composants avec des motifs de chemin pour \
             les lier aux décisions.",
        de: "Vorschläge erscheinen, wenn bestätigte Entscheidungen Dateien berühren oder \
             Abhängigkeiten hinzufügen. Erstelle Komponenten mit Pfadmustern, um sie mit \
             Entscheidungen zu verknüpfen.",
        it: "I suggerimenti compaiono quando decisioni confermate toccano file o aggiungono \
             dipendenze. Crea componenti con pattern di percorso per collegarli alle decisioni.",
        ja: "確定した決定がファイルに触れたり依存関係を追加したりすると、提案が表示されます。\
             パスパターンを持つコンポーネントを作成して、決定とリンクしましょう。",
        zh: "当已确认的决策涉及文件或添加依赖时,就会出现建议。\
             创建带路径模式的组件,即可将它们与决策关联。",
        ko: "확인된 결정이 파일을 건드리거나 의존성을 추가하면 제안이 나타나요. \
             경로 패턴이 있는 컴포넌트를 만들어 결정과 연결해 보세요.",
        ru: "Предложения появляются, когда подтверждённые решения затрагивают файлы или \
             добавляют зависимости. Создайте компоненты с шаблонами путей, чтобы связать их \
             с решениями." }
    /// Subtitle of the suggestions page.
    suggestions_subtitle {
        en: "What the captured work points to. Nothing enters the map without your confirmation.",
        pt: "O que o trabalho capturado indica. Nada entra no mapa sem a sua confirmação.",
        es: "Lo que indica el trabajo capturado. Nada entra en el mapa sin tu confirmación.",
        fr: "Ce qu’indique le travail capturé. Rien n’entre dans la carte sans votre confirmation.",
        de: "Worauf die erfasste Arbeit hindeutet. Nichts gelangt ohne deine Bestätigung in die \
             Karte.",
        it: "Ciò che indica il lavoro acquisito. Nulla entra nella mappa senza la tua conferma.",
        ja: "取り込んだ作業から読み取れることです。確定しない限り、マップには何も追加されません。",
        zh: "捕获的工作所显示的内容。未经你确认,任何内容都不会进入地图。",
        ko: "캡처한 작업이 가리키는 내용이에요. 확인하지 않으면 맵에 아무것도 들어가지 않아요.",
        ru: "На что указывает захваченная работа. Ничего не попадёт на карту без вашего \
             подтверждения." }
    /// Tag of a relation: the first decision depends on the second.
    relation_depends_on { en: "depends on", pt: "depende de", es: "depende de",
        fr: "dépend de", de: "hängt ab von", it: "dipende da", ja: "依存", zh: "依赖于",
        ko: "의존", ru: "зависит от" }
    /// Tag of a relation: the decisions conflict.
    relation_conflicts_with { en: "conflicts with", pt: "conflita com",
        es: "entra en conflicto con", fr: "est en conflit avec", de: "widerspricht",
        it: "è in conflitto con", ja: "競合", zh: "冲突", ko: "충돌",
        ru: "конфликтует с" }
    /// Tag of a relation: the first decision supersedes the second.
    relation_supersedes { en: "supersedes", pt: "substitui", es: "sustituye",
        fr: "remplace", de: "ersetzt", it: "sostituisce", ja: "置き換え", zh: "取代",
        ko: "대체", ru: "заменяет" }
    /// Caption beside the relation tag.
    between_two_decisions { en: "between two decisions", pt: "entre duas decisões",
        es: "entre dos decisiones", fr: "entre deux décisions",
        de: "zwischen zwei Entscheidungen", it: "tra due decisioni",
        ja: "2つの決定の間", zh: "两项决策之间", ko: "두 결정 사이",
        ru: "между двумя решениями" }
    /// Title of the section of suggested relations.
    relations_section_title { en: "Relations between decisions",
        pt: "Relações entre decisões", es: "Relaciones entre decisiones",
        fr: "Relations entre décisions", de: "Beziehungen zwischen Entscheidungen",
        it: "Relazioni tra decisioni", ja: "決定間の関係", zh: "决策之间的关系",
        ko: "결정 간 관계", ru: "Связи между решениями" }
    /// Explanation of the section of suggested relations.
    relations_section_hint {
        en: "Reading the confirmed decisions, xemnas noticed that some rely on, replace or \
             contradict others. Each card says which two decisions, what the relation is and \
             the passage where it shows up.",
        pt: "Ao ler as decisões confirmadas, o xemnas notou que algumas se apoiam, \
             substituem ou contradizem outras. Cada cartão diz quais duas decisões, \
             qual é a relação e o trecho em que ela aparece.",
        es: "Al leer las decisiones confirmadas, xemnas notó que algunas se apoyan en otras, \
             las sustituyen o las contradicen. Cada tarjeta indica qué dos decisiones, cuál \
             es la relación y el fragmento donde aparece.",
        fr: "En lisant les décisions confirmées, xemnas a remarqué que certaines s’appuient \
             sur d’autres, les remplacent ou les contredisent. Chaque carte indique quelles \
             deux décisions, quelle est la relation et l’extrait où elle apparaît.",
        de: "Beim Lesen der bestätigten Entscheidungen hat xemnas bemerkt, dass manche auf \
             anderen aufbauen, sie ersetzen oder ihnen widersprechen. Jede Karte nennt die \
             beiden Entscheidungen, die Beziehung und die Textstelle, in der sie auftaucht.",
        it: "Leggendo le decisioni confermate, xemnas ha notato che alcune si appoggiano ad \
             altre, le sostituiscono o le contraddicono. Ogni scheda indica quali due \
             decisioni, qual è la relazione e il passaggio in cui compare.",
        ja: "確定した決定を読み込んだところ、xemnasは、互いに依存したり、置き換えたり、\
             矛盾したりしている決定があることに気づきました。各カードには、どの2つの決定か、\
             どんな関係か、その根拠となる箇所が示されます。",
        zh: "xemnas 在阅读已确认的决策时发现,有些决策相互依赖、取代或与其他决策矛盾。\
             每张卡片会说明是哪两项决策、它们是什么关系,以及出处片段。",
        ko: "확인된 결정을 읽다가 xemnas는 서로 기대거나, 대체하거나, 모순되는 결정이 \
             있다는 걸 알아챘어요. 각 카드는 어떤 두 결정인지, 어떤 관계인지, 그 관계가 \
             나타난 구절을 보여 줘요.",
        ru: "Читая подтверждённые решения, xemnas заметил, что одни опираются на другие, \
             заменяют их или противоречат им. Карточка показывает, какие это два решения, \
             какая между ними связь и фрагмент, где она встречается." }
    /// Title of the section of suggested context.
    context_section_title { en: "Suggested context", pt: "Contexto sugerido",
        es: "Contexto sugerido", fr: "Contexte suggéré", de: "Vorgeschlagener Kontext",
        it: "Contesto suggerito", ja: "提案されたコンテキスト", zh: "建议的上下文",
        ko: "제안된 컨텍스트", ru: "Предложенный контекст" }
    /// Explanation of the section of suggested context.
    context_section_hint {
        en: "Rules, assumptions and constraints that seem to hold for the project, drawn from \
             the text of decisions already confirmed. Confirming turns the sentence into a \
             project rule (Context tab); rejecting discards it and it doesn't come back.",
        pt: "Regras, premissas e restrições que parecem valer para o projeto, tiradas do \
             texto de decisões já confirmadas. Confirmar transforma a frase em uma regra \
             do projeto (aba Contexto); rejeitar descarta a frase e ela não volta.",
        es: "Reglas, supuestos y restricciones que parecen valer para el proyecto, extraídos \
             del texto de decisiones ya confirmadas. Confirmar convierte la frase en una \
             regla del proyecto (pestaña Contexto); rechazar la descarta y no vuelve.",
        fr: "Règles, hypothèses et contraintes qui semblent valoir pour le projet, tirées du \
             texte de décisions déjà confirmées. Confirmer transforme la phrase en règle du \
             projet (onglet Contexte) ; rejeter l’écarte et elle ne revient pas.",
        de: "Regeln, Annahmen und Einschränkungen, die für das Projekt zu gelten scheinen, \
             entnommen aus dem Text bereits bestätigter Entscheidungen. Bestätigen macht den \
             Satz zu einer Projektregel (Tab Kontext); Ablehnen verwirft ihn, und er kommt \
             nicht wieder.",
        it: "Regole, presupposti e vincoli che sembrano valere per il progetto, ricavati dal \
             testo di decisioni già confermate. Confermare trasforma la frase in una regola \
             del progetto (scheda Contesto); rifiutare la scarta e non torna.",
        ja: "確定済みの決定の文章から取り出した、プロジェクトに当てはまりそうなルール・前提・\
             制約です。確定するとその文がプロジェクトのルールになり(コンテキストタブ)、\
             却下すると破棄されて再び表示されません。",
        zh: "从已确认决策的文字中提取出的、似乎适用于项目的规则、前提和约束。\
             确认会把这句话变成项目规则(“上下文”标签页);拒绝则会丢弃它,不会再出现。",
        ko: "이미 확인된 결정의 문장에서 뽑아낸, 프로젝트에 적용될 것 같은 규칙, 전제, \
             제약이에요. 확인하면 그 문장이 프로젝트 규칙이 되고(컨텍스트 탭), 거부하면 \
             버려져서 다시 나타나지 않아요.",
        ru: "Правила, допущения и ограничения, которые, судя по всему, действуют для \
             проекта, извлечённые из текста уже подтверждённых решений. Подтверждение \
             превращает фразу в правило проекта (вкладка «Контекст»), отклонение отбрасывает \
             её, и она не вернётся." }
    /// Error inside a suggested context that cannot be confirmed.
    qualification_error {
        en: "Couldn't read the qualifiers or scope. Confirmation unavailable.",
        pt: "Não foi possível ler qualificadores ou escopo. Confirmação indisponível.",
        es: "No se pudieron leer los calificadores o el alcance. Confirmación no disponible.",
        fr: "Impossible de lire les qualificateurs ou la portée. Confirmation indisponible.",
        de: "Qualifizierer oder Geltungsbereich konnten nicht gelesen werden. Bestätigung nicht \
             verfügbar.",
        it: "Impossibile leggere i qualificatori o l’ambito. Conferma non disponibile.",
        ja: "修飾条件または範囲を読み込めませんでした。確定できません。",
        zh: "无法读取限定条件或范围,暂时无法确认。",
        ko: "한정 조건이나 범위를 읽을 수 없어요. 확인할 수 없어요.",
        ru: "Не удалось прочитать квалификаторы или область. Подтверждение недоступно." }
    /// Line of a suggested context with no inherited scope.
    inherited_scope_missing { en: "Inherited scope not provided.",
        pt: "Escopo herdado não informado.", es: "Alcance heredado no indicado.",
        fr: "Portée héritée non indiquée.", de: "Geerbter Geltungsbereich nicht angegeben.",
        it: "Ambito ereditato non indicato.",
        ja: "継承された範囲は指定されていません。", zh: "未提供继承的范围。",
        ko: "상속된 범위가 지정되지 않았어요.", ru: "Унаследованная область не указана." }
    /// Effect of confirming a suggested context with no scope links.
    context_scope_none { en: "no scope links provided", pt: "sem vínculos de escopo informados",
        es: "sin vínculos de alcance indicados", fr: "aucun lien de portée indiqué",
        de: "keine Geltungsbereichs-Verknüpfungen angegeben",
        it: "nessun collegamento di ambito indicato",
        ja: "範囲のリンクは指定されていません", zh: "未提供范围关联",
        ko: "지정된 범위 연결 없음", ru: "связи с областью не указаны" }
    /// Title of the section of suggested links.
    links_section_title { en: "Suggested links", pt: "Vínculos sugeridos",
        es: "Vínculos sugeridos", fr: "Liens suggérés",
        de: "Vorgeschlagene Verknüpfungen", it: "Collegamenti suggeriti",
        ja: "提案されたリンク", zh: "建议的关联", ko: "제안된 연결",
        ru: "Предложенные ссылки" }
    /// Explanation of the section of suggested links.
    links_section_hint {
        en: "Decisions and rules that touched files or dependencies tied to a component or \
             technology on the Map. Confirming links the two: the decision shows up on the \
             component's page and goes along with whoever edits those files.",
        pt: "Decisões e regras que tocaram arquivos ou dependências ligados a um componente \
             ou tecnologia do Mapa. Confirmar liga os dois: a decisão aparece na página do \
             componente e acompanha quem edita aqueles arquivos.",
        es: "Decisiones y reglas que tocaron archivos o dependencias ligados a un componente \
             o tecnología del Mapa. Confirmar vincula ambos: la decisión aparece en la página \
             del componente y acompaña a quien edita esos archivos.",
        fr: "Décisions et règles qui ont touché des fichiers ou des dépendances liés à un \
             composant ou une technologie de la Carte. Confirmer lie les deux : la décision \
             apparaît sur la page du composant et accompagne quiconque modifie ces fichiers.",
        de: "Entscheidungen und Regeln, die Dateien oder Abhängigkeiten berührt haben, die zu \
             einer Komponente oder Technologie der Karte gehören. Bestätigen verknüpft beide: \
             Die Entscheidung erscheint auf der Seite der Komponente und begleitet alle, die \
             diese Dateien bearbeiten.",
        it: "Decisioni e regole che hanno toccato file o dipendenze legati a un componente o \
             a una tecnologia della Mappa. Confermare collega i due: la decisione compare \
             nella pagina del componente e accompagna chi modifica quei file.",
        ja: "マップのコンポーネントやテクノロジーに結び付くファイルや依存関係に触れた決定と\
             ルールです。確定すると両者がリンクされ、決定はコンポーネントのページに表示され、\
             それらのファイルを編集する人にも伝わります。",
        zh: "触及了与地图中某个组件或技术相关联的文件或依赖的决策和规则。\
             确认会把两者关联起来:该决策会出现在组件页面上,并伴随编辑这些文件的人。",
        ko: "맵의 컴포넌트나 기술에 연결된 파일 또는 의존성을 건드린 결정과 규칙이에요. \
             확인하면 둘이 연결되어, 결정이 컴포넌트 페이지에 나타나고 그 파일을 편집하는 \
             사람에게도 함께 전달돼요.",
        ru: "Решения и правила, затронувшие файлы или зависимости, связанные с компонентом или \
             технологией карты. Подтверждение связывает их: решение появляется на странице \
             компонента и сопровождает того, кто редактирует эти файлы." }
    /// Title of the section of suggested items.
    items_section_title { en: "Suggested items", pt: "Itens sugeridos",
        es: "Elementos sugeridos", fr: "Éléments suggérés",
        de: "Vorgeschlagene Elemente", it: "Elementi suggeriti", ja: "提案された項目",
        zh: "建议的条目", ko: "제안된 항목", ru: "Предложенные элементы" }
}

formats! {
    /// Line of a relation card: why the relation was suggested.
    reason_line(reason: &str) { en: "Why: {reason}", pt: "Por quê: {reason}",
        es: "Por qué: {reason}", fr: "Pourquoi : {reason}", de: "Warum: {reason}",
        it: "Perché: {reason}", ja: "理由: {reason}", zh: "原因:{reason}",
        ko: "이유: {reason}", ru: "Почему: {reason}" }
    /// Line of a suggested context: the scope it inherits.
    inherited_scope(scope: &str) { en: "Inherited scope: {scope}",
        pt: "Escopo herdado: {scope}", es: "Alcance heredado: {scope}",
        fr: "Portée héritée : {scope}", de: "Geerbter Geltungsbereich: {scope}",
        it: "Ambito ereditato: {scope}", ja: "継承された範囲: {scope}",
        zh: "继承的范围:{scope}", ko: "상속된 범위: {scope}",
        ru: "Унаследованная область: {scope}" }
    /// Effect of confirming a suggested context: where it applies.
    context_scope_some(names: &str) { en: "applies to {names}", pt: "vale em {names}",
        es: "vale en {names}", fr: "s’applique à {names}", de: "gilt für {names}",
        it: "vale per {names}", ja: "{names}に適用", zh: "适用于 {names}",
        ko: "{names}에 적용", ru: "действует для {names}" }
    /// Button: confirm every suggested link.
    confirm_all(count: usize) { en: "Confirm all {count}", pt: "Confirmar os {count}",
        es: "Confirmar los {count}", fr: "Confirmer les {count}",
        de: "Alle {count} bestätigen", it: "Conferma tutti i {count}",
        ja: "{count}件すべてを確定", zh: "确认全部 {count} 项", ko: "{count}개 모두 확인",
        ru: "Подтвердить все {count}" }
    /// Button: create every component the workspace declares.
    create_all_declared(count: usize) { en: "Create all {count} from the workspace",
        pt: "Criar os {count} do workspace", es: "Crear los {count} del workspace",
        fr: "Créer les {count} du workspace", de: "Alle {count} aus dem Workspace erstellen",
        it: "Crea tutti i {count} del workspace",
        ja: "ワークスペースの{count}件をすべて作成", zh: "创建工作区中的全部 {count} 项",
        ko: "워크스페이스의 {count}개 모두 만들기",
        ru: "Создать все {count} из рабочей области" }
    /// Lead of a suggested context, for languages other than Portuguese.
    context_lead_generic(question: &str) {
        en: "From the decision {BOLD}“{question}”{BOLD} xemnas drew out:",
        pt: "Da decisão {BOLD}“{question}”{BOLD} o xemnas tirou:",
        es: "De la decisión {BOLD}“{question}”{BOLD} xemnas extrajo:",
        fr: "De la décision {BOLD}“{question}”{BOLD}, xemnas a extrait :",
        de: "Aus der Entscheidung {BOLD}„{question}“{BOLD} hat xemnas Folgendes abgeleitet:",
        it: "Dalla decisione {BOLD}“{question}”{BOLD} xemnas ha ricavato:",
        ja: "決定 {BOLD}「{question}」{BOLD} から、xemnasは次を抽出しました:",
        zh: "xemnas 从决策 {BOLD}“{question}”{BOLD} 中提取出:",
        ko: "결정 {BOLD}“{question}”{BOLD}에서 xemnas가 이걸 뽑아냈어요:",
        ru: "Из решения {BOLD}«{question}»{BOLD} xemnas извлёк:" }
    /// Effect of a suggested context, for languages other than Portuguese.
    context_effect_generic(scope: &str) {
        en: "it becomes a project rule ({scope}) and the agent starts receiving it in context.",
        pt: "vira uma regra do projeto ({scope}) e o agente passa a recebê-la no contexto.",
        es: "se convierte en una regla del proyecto ({scope}) y el agente empieza a recibirla \
             en el contexto.",
        fr: "elle devient une règle du projet ({scope}) et l’agent commence à la recevoir \
             dans le contexte.",
        de: "sie wird zu einer Regel des Projekts ({scope}), und der Agent erhält sie ab dann \
             im Kontext.",
        it: "diventa una regola del progetto ({scope}) e l’agente inizia a riceverla nel \
             contesto.",
        ja: "プロジェクトのルールになり({scope})、エージェントがコンテキストで受け取るようになります。",
        zh: "它会成为项目规则({scope}),智能体从此会在上下文中收到它。",
        ko: "프로젝트 규칙이 되고({scope}) 에이전트가 컨텍스트로 받게 돼요.",
        ru: "становится правилом проекта ({scope}), и агент начинает получать её в контексте." }
}

/// "uma" or "um" for a kind of rule, as the Portuguese sentence needs it.
fn claim_article_pt(kind: &str) -> &'static str {
    match kind {
        "goal" => "um",
        _ => "uma",
    }
}

/// Lead sentence of a suggested context: which decision it was drawn from.
/// Portuguese names the kind of rule, which needs its gender; the other
/// languages leave the kind to the tag beside the sentence.
pub fn context_lead(question: &str, kind: &str) -> String {
    if super::current() != Language::Portuguese {
        return context_lead_generic(question);
    }
    format!(
        "Da decisão {BOLD}“{question}”{BOLD} o xemnas tirou {} {}:",
        claim_article_pt(kind),
        claim_label(kind).to_lowercase()
    )
}

/// What confirming a suggested context changes; `scope` says where it holds.
pub fn context_effect(kind: &str, scope: &str) -> String {
    if super::current() != Language::Portuguese {
        return context_effect_generic(scope);
    }
    let article = claim_article_pt(kind);
    let pronoun = if article == "um" { "lo" } else { "la" };
    format!(
        "vira {article} {} do projeto ({scope}) e o agente passa a recebê-{pronoun} no contexto.",
        claim_label(kind).to_lowercase()
    )
}

// ---- entity page -----------------------------------------------------------

strings! {
    /// Button: link a decision to the open item.
    link_decision { en: "Link decision", pt: "Vincular decisão", es: "Vincular decisión",
        fr: "Lier une décision", de: "Entscheidung verknüpfen", it: "Collega decisione",
        ja: "決定をリンク", zh: "关联决策", ko: "결정 연결", ru: "Связать решение" }
    /// Button: link a rule to the open item.
    link_rule { en: "Link rule", pt: "Vincular regra", es: "Vincular regla",
        fr: "Lier une règle", de: "Regel verknüpfen", it: "Collega regola",
        ja: "ルールをリンク", zh: "关联规则", ko: "규칙 연결", ru: "Связать правило" }
    /// Button: choose the component that contains the open one.
    part_of_button { en: "Part of…", pt: "Faz parte de…", es: "Forma parte de…",
        fr: "Fait partie de…", de: "Teil von…", it: "Fa parte di…", ja: "所属先…",
        zh: "隶属于…", ko: "소속…", ru: "Входит в…" }
    /// Button: edit the open item.
    edit { en: "Edit", pt: "Editar", es: "Editar", fr: "Modifier", de: "Bearbeiten",
        it: "Modifica", ja: "編集", zh: "编辑", ko: "편집", ru: "Изменить" }
    /// Button: retire the open item.
    retire { en: "Retire", pt: "Aposentar", es: "Retirar", fr: "Retirer",
        de: "Stilllegen", it: "Ritira", ja: "廃止", zh: "停用", ko: "폐기",
        ru: "Списать" }
    /// Button: confirm retiring the open item.
    retire_item { en: "Retire item", pt: "Aposentar item", es: "Retirar elemento",
        fr: "Retirer l’élément", de: "Element stilllegen", it: "Ritira elemento",
        ja: "項目を廃止", zh: "停用条目", ko: "항목 폐기", ru: "Списать элемент" }
    /// Warning before retiring an item.
    retire_warning {
        en: "Retiring takes the item off the map and undoes its links. Nothing is deleted: \
             the timeline keeps showing what held.",
        pt: "Aposentar tira o item do mapa e desfaz os vínculos dele. Nada é apagado: a linha \
             do tempo continua mostrando o que valeu.",
        es: "Retirar quita el elemento del mapa y deshace sus vínculos. No se borra nada: la \
             línea de tiempo sigue mostrando lo que valió.",
        fr: "Retirer enlève l’élément de la carte et défait ses liens. Rien n’est supprimé : \
             la chronologie continue d’afficher ce qui valait.",
        de: "Stilllegen nimmt das Element aus der Karte und löst seine Verknüpfungen. Nichts \
             wird gelöscht: Die Zeitleiste zeigt weiterhin, was galt.",
        it: "Ritirare toglie l’elemento dalla mappa e scioglie i suoi collegamenti. Non viene \
             cancellato nulla: la linea temporale continua a mostrare ciò che valeva.",
        ja: "廃止すると、項目はマップから外れ、リンクも解除されます。何も削除されず、\
             タイムラインには有効だった内容が引き続き表示されます。",
        zh: "停用会把条目从地图中移除,并解除它的关联。不会删除任何内容:时间线仍会显示当时生效的内容。",
        ko: "폐기하면 항목이 맵에서 빠지고 연결도 해제돼요. 아무것도 삭제되지 않아요. \
             타임라인에는 유효했던 내용이 계속 보여요.",
        ru: "Списание убирает элемент с карты и разрывает его связи. Ничего не удаляется: \
             хронология по-прежнему показывает то, что действовало." }
    /// Section: what surrounds the open item.
    neighborhood { en: "Neighborhood", pt: "Vizinhança", es: "Vecindad",
        fr: "Voisinage", de: "Umgebung", it: "Vicinanza", ja: "周辺", zh: "邻近关系",
        ko: "이웃", ru: "Окружение" }
    /// Section: decisions in effect.
    decisions_in_force { en: "Decisions in effect", pt: "Decisões em vigor",
        es: "Decisiones vigentes", fr: "Décisions en vigueur",
        de: "Entscheidungen in Kraft", it: "Decisioni in vigore", ja: "有効な決定",
        zh: "生效中的决策", ko: "유효한 결정", ru: "Действующие решения" }
    /// Section: conflicts on the open item.
    conflicts_here { en: "Conflicts on this item", pt: "Conflitos neste item",
        es: "Conflictos en este elemento", fr: "Conflits sur cet élément",
        de: "Konflikte bei diesem Element", it: "Conflitti su questo elemento",
        ja: "この項目の競合", zh: "此条目的冲突", ko: "이 항목의 충돌",
        ru: "Конфликты в этом элементе" }
    /// Section: rules that apply to the open item.
    rules_that_apply { en: "Rules that apply", pt: "Regras que se aplicam",
        es: "Reglas que se aplican", fr: "Règles applicables", de: "Geltende Regeln",
        it: "Regole che si applicano", ja: "適用されるルール", zh: "适用的规则",
        ko: "적용되는 규칙", ru: "Применяемые правила" }
    /// Section: the component's place among others.
    structure { en: "Structure", pt: "Estrutura", es: "Estructura", fr: "Structure",
        de: "Struktur", it: "Struttura", ja: "構造", zh: "结构", ko: "구조",
        ru: "Структура" }
    /// List title: the component that contains this one.
    part_of_list { en: "Part of", pt: "Faz parte de", es: "Forma parte de",
        fr: "Fait partie de", de: "Teil von", it: "Fa parte di", ja: "所属先",
        zh: "隶属于", ko: "소속", ru: "Входит в" }
    /// List title: the parts of this component.
    parts { en: "Parts", pt: "Partes", es: "Partes", fr: "Parties", de: "Teile",
        it: "Parti", ja: "パート", zh: "部分", ko: "파트", ru: "Части" }
    /// Section: decisions that depend on the open item.
    impact_title { en: "Impact: decisions that depend on this item",
        pt: "Impacto: decisões que dependem deste item",
        es: "Impacto: decisiones que dependen de este elemento",
        fr: "Impact : décisions qui dépendent de cet élément",
        de: "Auswirkung: Entscheidungen, die von diesem Element abhängen",
        it: "Impatto: decisioni che dipendono da questo elemento",
        ja: "影響: この項目に依存する決定", zh: "影响:依赖此条目的决策",
        ko: "영향: 이 항목에 의존하는 결정",
        ru: "Влияние: решения, зависящие от этого элемента" }
    /// Empty list of linked decisions on an item.
    no_linked_decisions {
        en: "No linked decisions. Link one or confirm suggestions.",
        pt: "Nenhuma decisão ligada. Vincule uma ou confirme sugestões.",
        es: "Ninguna decisión vinculada. Vincula una o confirma sugerencias.",
        fr: "Aucune décision liée. Liez-en une ou confirmez des suggestions.",
        de: "Keine Entscheidung verknüpft. Verknüpfe eine oder bestätige Vorschläge.",
        it: "Nessuna decisione collegata. Collegane una o conferma i suggerimenti.",
        ja: "リンクされた決定はありません。決定をリンクするか、提案を確定してください。",
        zh: "没有关联的决策。请关联一项,或确认建议。",
        ko: "연결된 결정이 없어요. 하나를 연결하거나 제안을 확인해 보세요.",
        ru: "Нет связанных решений. Свяжите решение или подтвердите предложения." }
    /// Empty side of the neighborhood diagram: no decisions.
    no_linked_decisions_short { en: "No linked decisions.",
        pt: "Nenhuma decisão ligada.", es: "Ninguna decisión vinculada.",
        fr: "Aucune décision liée.", de: "Keine Entscheidung verknüpft.",
        it: "Nessuna decisione collegata.", ja: "リンクされた決定はありません。",
        zh: "没有关联的决策。", ko: "연결된 결정이 없어요.",
        ru: "Нет связанных решений." }
    /// Empty list of linked rules.
    no_linked_rules { en: "No linked rules.", pt: "Nenhuma regra ligada.",
        es: "Ninguna regla vinculada.", fr: "Aucune règle liée.",
        de: "Keine Regel verknüpft.", it: "Nessuna regola collegata.",
        ja: "リンクされたルールはありません。", zh: "没有关联的规则。",
        ko: "연결된 규칙이 없어요.", ru: "Нет связанных правил." }
    /// Picker title: choose a decision to link.
    pick_decision { en: "Choose the decision", pt: "Escolha a decisão",
        es: "Elige la decisión", fr: "Choisissez la décision", de: "Wähle die Entscheidung",
        it: "Scegli la decisione", ja: "決定を選んでください", zh: "选择决策",
        ko: "결정을 골라 주세요", ru: "Выберите решение" }
    /// Picker title: choose a rule to link.
    pick_rule { en: "Choose the rule", pt: "Escolha a regra", es: "Elige la regla",
        fr: "Choisissez la règle", de: "Wähle die Regel", it: "Scegli la regola",
        ja: "ルールを選んでください", zh: "选择规则", ko: "규칙을 골라 주세요",
        ru: "Выберите правило" }
    /// Picker title: choose the parent component.
    pick_parent { en: "Choose the component that contains this one",
        pt: "Escolha o componente que contém este",
        es: "Elige el componente que contiene a este",
        fr: "Choisissez le composant qui contient celui-ci",
        de: "Wähle die Komponente, die diese enthält",
        it: "Scegli il componente che contiene questo",
        ja: "これを含むコンポーネントを選んでください",
        zh: "选择包含此组件的组件", ko: "이것을 포함하는 컴포넌트를 골라 주세요",
        ru: "Выберите компонент, который содержит этот" }
    /// Picker: nothing left to link.
    nothing_to_link { en: "Nothing to link here.", pt: "Nada para vincular aqui.",
        es: "Nada que vincular aquí.", fr: "Rien à lier ici.",
        de: "Hier gibt es nichts zu verknüpfen.", it: "Niente da collegare qui.",
        ja: "ここにはリンクできるものがありません。", zh: "这里没有可关联的内容。",
        ko: "여기에는 연결할 항목이 없어요.", ru: "Здесь нечего связывать." }
}

formats! {
    /// Line under a rule: its kind and since when it holds.
    rule_since(kind: &str, date: &str) { en: "{kind} · since {date}",
        pt: "{kind} · desde {date}", es: "{kind} · desde {date}",
        fr: "{kind} · depuis le {date}", de: "{kind} · seit {date}",
        it: "{kind} · dal {date}", ja: "{kind} · {date}から", zh: "{kind} · 自 {date}",
        ko: "{kind} · {date}부터", ru: "{kind} · с {date}" }
    /// Line under an item: what else it is also called.
    also_called(aliases: &str) { en: "Also called {aliases}",
        pt: "Também chamado de {aliases}", es: "También llamado {aliases}",
        fr: "Aussi appelé {aliases}", de: "Auch genannt: {aliases}",
        it: "Chiamato anche {aliases}", ja: "別名: {aliases}", zh: "又称:{aliases}",
        ko: "다른 이름: {aliases}", ru: "Также называется: {aliases}" }
    /// Label of the neighborhood diagram.
    diagram_label(name: &str, left: usize, right: usize) {
        en: "{name}: decisions on the left: {left}, rules on the right: {right}",
        pt: "{name}: {left} decisões à esquerda, {right} regras à direita",
        es: "{name}: decisiones a la izquierda: {left}, reglas a la derecha: {right}",
        fr: "{name} : décisions à gauche : {left}, règles à droite : {right}",
        de: "{name}: Entscheidungen links: {left}, Regeln rechts: {right}",
        it: "{name}: decisioni a sinistra: {left}, regole a destra: {right}",
        ja: "{name}: 左に決定{left}件、右にルール{right}件",
        zh: "{name}:左侧决策 {left} 项,右侧规则 {right} 项",
        ko: "{name}: 왼쪽 결정 {left}개, 오른쪽 규칙 {right}개",
        ru: "{name}: решений слева: {left}, правил справа: {right}" }
    /// Note under the neighborhood diagram: what it leaves out.
    more_in_lists(hidden: usize) { en: "{hidden} more in the lists below.",
        pt: "Mais {hidden} nas listas abaixo.", es: "{hidden} más en las listas de abajo.",
        fr: "{hidden} de plus dans les listes ci-dessous.",
        de: "{hidden} weitere in den Listen unten.",
        it: "Altri {hidden} nelle liste qui sotto.",
        ja: "下のリストにさらに{hidden}件あります。",
        zh: "下方列表中还有 {hidden} 项。", ko: "아래 목록에 {hidden}개가 더 있어요.",
        ru: "Ещё {hidden} в списках ниже." }
}

// ---- file lens -------------------------------------------------------------

strings! {
    /// Subtitle of the file lens page.
    file_lens_subtitle {
        en: "What holds for a file: the components that contain it, the decisions in effect \
             and the rules linked to them.",
        pt: "O que vale para um arquivo: os componentes que o contêm, as decisões em vigor \
             e as regras ligadas a eles.",
        es: "Lo que vale para un archivo: los componentes que lo contienen, las decisiones \
             vigentes y las reglas vinculadas a ellos.",
        fr: "Ce qui vaut pour un fichier : les composants qui le contiennent, les décisions \
             en vigueur et les règles qui leur sont liées.",
        de: "Was für eine Datei gilt: die Komponenten, die sie enthalten, die Entscheidungen \
             in Kraft und die mit ihnen verknüpften Regeln.",
        it: "Cosa vale per un file: i componenti che lo contengono, le decisioni in vigore \
             e le regole collegate a essi.",
        ja: "ファイルに有効な内容です。そのファイルを含むコンポーネント、有効な決定、\
             それらにリンクされたルールを表示します。",
        zh: "适用于某个文件的内容:包含它的组件、生效中的决策,以及与它们关联的规则。",
        ko: "파일에 적용되는 내용이에요. 그 파일을 포함하는 컴포넌트, 유효한 결정, \
             그리고 이들에 연결된 규칙을 보여 줘요.",
        ru: "Что действует для файла: компоненты, которые его содержат, действующие решения и \
             связанные с ними правила." }
    /// Label of the path field in the file lens.
    relative_path { en: "Path relative to the project", pt: "Caminho relativo ao projeto",
        es: "Ruta relativa al proyecto", fr: "Chemin relatif au projet",
        de: "Pfad relativ zum Projekt", it: "Percorso relativo al progetto",
        ja: "プロジェクトからの相対パス", zh: "相对于项目的路径",
        ko: "프로젝트 기준 상대 경로", ru: "Путь относительно проекта" }
    /// Button: read what holds for the file.
    show_what_applies { en: "See what applies", pt: "Ver o que vale",
        es: "Ver qué vale", fr: "Voir ce qui vaut", de: "Ansehen, was gilt",
        it: "Vedi cosa vale", ja: "有効な内容を見る", zh: "查看生效内容",
        ko: "적용되는 내용 보기", ru: "Показать, что действует" }
    /// Section: files of the latest decisions, as quick picks.
    recent_decision_files { en: "Files from recent decisions",
        pt: "Arquivos das últimas decisões", es: "Archivos de las últimas decisiones",
        fr: "Fichiers des dernières décisions", de: "Dateien der letzten Entscheidungen",
        it: "File delle ultime decisioni", ja: "最近の決定のファイル",
        zh: "近期决策涉及的文件", ko: "최근 결정의 파일",
        ru: "Файлы последних решений" }
    /// File lens: no decision in effect on the components that cover the file.
    no_decisions_for_components {
        en: "No decisions in effect linked to these components.",
        pt: "Nenhuma decisão em vigor ligada a esses componentes.",
        es: "Ninguna decisión vigente vinculada a estos componentes.",
        fr: "Aucune décision en vigueur liée à ces composants.",
        de: "Keine Entscheidung in Kraft mit diesen Komponenten verknüpft.",
        it: "Nessuna decisione in vigore collegata a questi componenti.",
        ja: "これらのコンポーネントにリンクされた有効な決定はありません。",
        zh: "这些组件没有关联的生效决策。",
        ko: "이 컴포넌트에 연결된 유효한 결정이 없어요.",
        ru: "Нет действующих решений, связанных с этими компонентами." }
    /// File lens section: rules linked to the components.
    lens_rules { en: "Rules", pt: "Regras", es: "Reglas", fr: "Règles", de: "Regeln",
        it: "Regole", ja: "ルール", zh: "规则", ko: "규칙", ru: "Правила" }
    /// Subtitle of the timeline page.
    timeline_subtitle {
        en: "What started and stopped holding in the project, from newest to oldest.",
        pt: "O que passou a valer e o que deixou de valer no projeto, do mais recente ao mais \
             antigo.",
        es: "Lo que pasó a valer y lo que dejó de valer en el proyecto, del más reciente al más \
             antiguo.",
        fr: "Ce qui a commencé et cessé de valoir dans le projet, du plus récent au plus ancien.",
        de: "Was im Projekt zu gelten begann und was nicht mehr gilt, vom neuesten zum ältesten.",
        it: "Ciò che ha iniziato e smesso di valere nel progetto, dal più recente al più vecchio.",
        ja: "プロジェクトで有効になったこと、無効になったことを新しい順に表示します。",
        zh: "项目中开始生效和不再生效的内容,按从新到旧排列。",
        ko: "프로젝트에서 효력이 생기거나 사라진 내용을 최신순으로 보여 줘요.",
        ru: "Что начало и перестало действовать в проекте, от новых к старым." }
}

formats! {
    /// File lens: no component covers the path.
    no_component_covers(path: &str) {
        en: "No component covers {path}. Create one with a path pattern that includes it.",
        pt: "Nenhum componente cobre {path}. Crie um com um padrão de caminho que o inclua.",
        es: "Ningún componente cubre {path}. Crea uno con un patrón de ruta que lo incluya.",
        fr: "Aucun composant ne couvre {path}. Créez-en un avec un motif de chemin qui l’inclut.",
        de: "Keine Komponente deckt {path} ab. Erstelle eine mit einem Pfadmuster, das sie \
             einschließt.",
        it: "Nessun componente copre {path}. Creane uno con un pattern di percorso che lo includa.",
        ja: "{path}を対象とするコンポーネントはありません。これを含むパスパターンで作成してください。",
        zh: "没有组件覆盖 {path}。请创建一个路径模式包含它的组件。",
        ko: "{path}을(를) 포함하는 컴포넌트가 없어요. 이 경로를 포함하는 경로 패턴으로 하나 만들어 보세요.",
        ru: "Ни один компонент не охватывает {path}. Создайте компонент с шаблоном пути, который \
             его включает." }
}

// ---- form ------------------------------------------------------------------

strings! {
    /// Description of the component kind in the form.
    component_blurb { en: "A part of the project, found by paths",
        pt: "Uma parte do projeto, achada por caminhos",
        es: "Una parte del proyecto, hallada por rutas",
        fr: "Une partie du projet, repérée par chemins",
        de: "Ein Teil des Projekts, über Pfade gefunden",
        it: "Una parte del progetto, individuata tramite percorsi",
        ja: "パスで見つかるプロジェクトの一部", zh: "项目的一部分,通过路径识别",
        ko: "경로로 찾는 프로젝트의 일부", ru: "Часть проекта, находимая по путям" }
    /// Description of the technology kind in the form.
    technology_blurb { en: "A language, library or service in use",
        pt: "Linguagem, biblioteca ou serviço usado",
        es: "Lenguaje, biblioteca o servicio usado",
        fr: "Langage, bibliothèque ou service utilisé",
        de: "Verwendete Sprache, Bibliothek oder verwendeter Dienst",
        it: "Linguaggio, libreria o servizio usato",
        ja: "使用している言語、ライブラリ、サービス", zh: "所用的语言、库或服务",
        ko: "사용 중인 언어, 라이브러리 또는 서비스",
        ru: "Используемый язык, библиотека или сервис" }
    /// Form title when editing.
    edit_item { en: "Edit item", pt: "Editar item", es: "Editar elemento",
        fr: "Modifier l’élément", de: "Element bearbeiten", it: "Modifica elemento",
        ja: "項目を編集", zh: "编辑条目", ko: "항목 편집", ru: "Изменить элемент" }
    /// Form title when creating.
    new_map_item { en: "New map item", pt: "Novo item do mapa",
        es: "Nuevo elemento del mapa", fr: "Nouvel élément de la carte",
        de: "Neues Kartenelement", it: "Nuovo elemento della mappa",
        ja: "マップの新しい項目", zh: "新建地图条目", ko: "새 맵 항목",
        ru: "Новый элемент карты" }
    /// Subtitle of the form.
    form_subtitle {
        en: "Components tie decisions to project paths; technologies, to the dependencies \
             they adopt.",
        pt: "Componentes ligam decisões a caminhos do projeto; tecnologias, às dependências \
             que elas adotam.",
        es: "Los componentes vinculan decisiones a rutas del proyecto; las tecnologías, a las \
             dependencias que adoptan.",
        fr: "Les composants relient des décisions à des chemins du projet ; les technologies, \
             aux dépendances qu’elles adoptent.",
        de: "Komponenten verbinden Entscheidungen mit Projektpfaden; Technologien mit den \
             Abhängigkeiten, die sie übernehmen.",
        it: "I componenti collegano le decisioni ai percorsi del progetto; le tecnologie, \
             alle dipendenze che adottano.",
        ja: "コンポーネントは決定をプロジェクトのパスに結び付け、テクノロジーは決定が採用する\
             依存関係に結び付けます。",
        zh: "组件将决策与项目路径关联起来;技术则关联决策所采用的依赖。",
        ko: "컴포넌트는 결정을 프로젝트 경로에 연결하고, 기술은 결정이 채택한 의존성에 \
             연결해요.",
        ru: "Компоненты связывают решения с путями проекта, а технологии — с зависимостями, \
             которые те принимают." }
    /// Accessible name of the group that picks the kind of item.
    kind_group { en: "Type", pt: "Tipo", es: "Tipo", fr: "Type", de: "Typ", it: "Tipo",
        ja: "種類", zh: "类型", ko: "유형", ru: "Тип" }
    /// Form field: name.
    name_label { en: "Name", pt: "Nome", es: "Nombre", fr: "Nom", de: "Name",
        it: "Nome", ja: "名前", zh: "名称", ko: "이름", ru: "Название" }
    /// Hint of the name field.
    name_hint { en: "What the team calls this part.", pt: "Como o time chama esta parte.",
        es: "Cómo llama el equipo a esta parte.", fr: "Comment l’équipe appelle cette partie.",
        de: "Wie das Team diesen Teil nennt.", it: "Come il team chiama questa parte.",
        ja: "チームがこの部分を何と呼んでいるか。", zh: "团队如何称呼这一部分。",
        ko: "팀에서 이 부분을 부르는 이름이에요.", ru: "Как команда называет эту часть." }
    /// Form field: path patterns.
    patterns_label { en: "Path patterns", pt: "Padrões de caminho",
        es: "Patrones de ruta", fr: "Motifs de chemin", de: "Pfadmuster",
        it: "Pattern di percorso", ja: "パスパターン", zh: "路径模式",
        ko: "경로 패턴", ru: "Шаблоны путей" }
    /// Hint of the path patterns field.
    patterns_hint {
        en: "Comma-separated. * matches within a folder, ** crosses folders; with no \
             wildcard, the whole folder matches.",
        pt: "Separados por vírgula. * vale dentro de uma pasta, ** atravessa pastas; sem \
             curinga, vale a pasta inteira.",
        es: "Separados por comas. * vale dentro de una carpeta, ** atraviesa carpetas; sin \
             comodín, vale la carpeta entera.",
        fr: "Séparés par des virgules. * s’applique dans un dossier, ** traverse les dossiers ; \
             sans joker, tout le dossier est concerné.",
        de: "Durch Kommas getrennt. * gilt innerhalb eines Ordners, ** reicht über Ordner \
             hinweg; ohne Platzhalter gilt der ganze Ordner.",
        it: "Separati da virgole. * vale dentro una cartella, ** attraversa le cartelle; \
             senza carattere jolly vale l’intera cartella.",
        ja: "カンマ区切り。*は1つのフォルダ内、**はフォルダをまたいで一致します。\
             ワイルドカードがなければフォルダ全体が対象です。",
        zh: "用逗号分隔。* 匹配单个文件夹内,** 可跨文件夹;不带通配符则匹配整个文件夹。",
        ko: "쉼표로 구분해요. *는 한 폴더 안에서, **는 폴더를 가로질러 일치하고, \
             와일드카드가 없으면 폴더 전체에 적용돼요.",
        ru: "Через запятую. * действует внутри папки, ** проходит через папки; без \
             подстановочного знака подходит вся папка." }
    /// Form field: aliases.
    aliases_label { en: "Aliases", pt: "Apelidos", es: "Alias", fr: "Alias",
        de: "Aliasse", it: "Alias", ja: "別名", zh: "别名", ko: "별칭",
        ru: "Псевдонимы" }
    /// Hint of the aliases field.
    aliases_hint {
        en: "Other names, comma-separated. They resolve dependencies with a different name.",
        pt: "Outros nomes, separados por vírgula. Resolvem dependências com nome diferente.",
        es: "Otros nombres, separados por comas. Resuelven dependencias con otro nombre.",
        fr: "Autres noms, séparés par des virgules. Ils résolvent les dépendances portant un nom \
             différent.",
        de: "Andere Namen, durch Kommas getrennt. Sie lösen Abhängigkeiten mit abweichendem \
             Namen auf.",
        it: "Altri nomi, separati da virgole. Risolvono dipendenze con un nome diverso.",
        ja: "別の名前をカンマ区切りで入力します。名前が異なる依存関係の解決に使われます。",
        zh: "其他名称,用逗号分隔。用于匹配名称不同的依赖。",
        ko: "다른 이름을 쉼표로 구분해요. 이름이 다른 의존성을 찾는 데 쓰여요.",
        ru: "Другие названия через запятую. Помогают находить зависимости с другим именем." }
    /// Form field: description.
    description_label { en: "Description", pt: "Descrição", es: "Descripción",
        fr: "Description", de: "Beschreibung", it: "Descrizione", ja: "説明",
        zh: "描述", ko: "설명", ru: "Описание" }
    /// Hint of the description field.
    optional { en: "Optional.", pt: "Opcional.", es: "Opcional.", fr: "Facultatif.",
        de: "Optional.", it: "Facoltativo.", ja: "任意。", zh: "可选。",
        ko: "선택 사항이에요.", ru: "Необязательно." }
}

// ---- overview and graph ----------------------------------------------------

strings! {
    /// Layout switch: the blocks drawing.
    layout_blocks { en: "Blocks", pt: "Blocos", es: "Bloques", fr: "Blocs",
        de: "Blöcke", it: "Blocchi", ja: "ブロック", zh: "区块", ko: "블록",
        ru: "Блоки" }
    /// Layout switch: the graph drawing.
    layout_graph { en: "Graph", pt: "Grafo", es: "Grafo", fr: "Graphe", de: "Graph",
        it: "Grafo", ja: "グラフ", zh: "图谱", ko: "그래프", ru: "Граф" }
    /// Title of the overview page.
    project_map { en: "Project map", pt: "Mapa do projeto", es: "Mapa del proyecto",
        fr: "Carte du projet", de: "Projektkarte", it: "Mappa del progetto",
        ja: "プロジェクトマップ", zh: "项目地图", ko: "프로젝트 맵", ru: "Карта проекта" }
    /// Subtitle of the graph layout.
    graph_subtitle {
        en: "Each island is a component with what holds in it. Hover to see the links; \
             click to open.",
        pt: "Cada ilha é um componente com o que vale nele. Passe o mouse para ver as \
             ligações; clique para abrir.",
        es: "Cada isla es un componente con lo que vale en él. Pasa el ratón para ver los \
             vínculos; haz clic para abrir.",
        fr: "Chaque îlot est un composant avec ce qui y vaut. Survolez pour voir les liens ; \
             cliquez pour ouvrir.",
        de: "Jede Insel ist eine Komponente mit dem, was in ihr gilt. Fahre mit der Maus \
             darüber, um die Verknüpfungen zu sehen; klicke zum Öffnen.",
        it: "Ogni isola è un componente con ciò che vale al suo interno. Passa il mouse per \
             vedere i collegamenti; clicca per aprire.",
        ja: "各島は、有効な内容を持つ1つのコンポーネントです。マウスを重ねるとリンクが見え、\
             クリックすると開きます。",
        zh: "每个岛是一个组件及其中生效的内容。悬停可查看关联,点击可打开。",
        ko: "각 섬은 하나의 컴포넌트와 그 안에서 유효한 내용이에요. 마우스를 올리면 연결이 \
             보이고, 클릭하면 열려요.",
        ru: "Каждый остров — компонент с тем, что в нём действует. Наведите курсор, чтобы \
             увидеть связи; щёлкните, чтобы открыть." }
    /// Subtitle of the overview when the map is empty.
    overview_empty_subtitle {
        en: "Still empty. Components tie decisions to parts of the code; start with the \
             suggestions, which come from the files the decisions touched.",
        pt: "Ainda vazio. Componentes ligam as decisões às partes do código; comece pelas \
             sugestões, que vêm dos arquivos que as decisões tocaram.",
        es: "Aún vacío. Los componentes vinculan las decisiones con partes del código; \
             empieza por las sugerencias, que vienen de los archivos que tocaron las decisiones.",
        fr: "Encore vide. Les composants relient les décisions aux parties du code ; \
             commencez par les suggestions, qui viennent des fichiers touchés par les décisions.",
        de: "Noch leer. Komponenten verbinden Entscheidungen mit Teilen des Codes; beginne mit \
             den Vorschlägen, die aus den von den Entscheidungen berührten Dateien stammen.",
        it: "Ancora vuota. I componenti collegano le decisioni alle parti del codice; parti \
             dai suggerimenti, che arrivano dai file toccati dalle decisioni.",
        ja: "まだ空です。コンポーネントは決定をコードの各部分に結び付けます。まず、決定が\
             触れたファイルから作られる提案から始めましょう。",
        zh: "目前还是空的。组件把决策与代码的各部分关联起来;可以先从建议开始,\
             它们来自决策涉及的文件。",
        ko: "아직 비어 있어요. 컴포넌트는 결정을 코드의 각 부분에 연결해요. 결정이 건드린 \
             파일에서 나온 제안부터 시작해 보세요.",
        ru: "Пока пусто. Компоненты связывают решения с частями кода; начните с предложений — \
             они берутся из файлов, которых коснулись решения." }
    /// Button: open the suggestions from an empty map.
    see_suggestions { en: "See suggestions", pt: "Ver sugestões", es: "Ver sugerencias",
        fr: "Voir les suggestions", de: "Vorschläge ansehen", it: "Vedi suggerimenti",
        ja: "提案を見る", zh: "查看建议", ko: "제안 보기", ru: "Показать предложения" }
    /// Subtitle of the blocks layout.
    blocks_subtitle {
        en: "The architecture drawn by confirmed decisions. Open a block to see what holds there.",
        pt: "A arquitetura desenhada pelas decisões confirmadas. Abra um bloco para ver o que \
             vale ali.",
        es: "La arquitectura dibujada por las decisiones confirmadas. Abre un bloque para ver lo \
             que vale allí.",
        fr: "L’architecture dessinée par les décisions confirmées. Ouvrez un bloc pour voir ce \
             qui y vaut.",
        de: "Die durch bestätigte Entscheidungen gezeichnete Architektur. Öffne einen Block, um \
             zu sehen, was dort gilt.",
        it: "L’architettura disegnata dalle decisioni confermate. Apri un blocco per vedere cosa \
             vale lì.",
        ja: "確定した決定が描くアーキテクチャです。ブロックを開くと、そこで有効な内容が見られます。",
        zh: "由已确认的决策勾勒出的架构。打开一个区块,查看其中生效的内容。",
        ko: "확인된 결정이 그려 낸 아키텍처예요. 블록을 열면 그곳에서 유효한 내용을 볼 수 있어요.",
        ru: "Архитектура, нарисованная подтверждёнными решениями. Откройте блок, чтобы увидеть, \
             что там действует." }
    /// Legend of the conflict dot.
    conflict_legend { en: "decisions in conflict", pt: "decisões em conflito",
        es: "decisiones en conflicto", fr: "décisions en conflit",
        de: "Entscheidungen im Konflikt", it: "decisioni in conflitto",
        ja: "競合している決定", zh: "存在冲突的决策", ko: "충돌하는 결정",
        ru: "решения в конфликте" }
}

formats! {
    /// Block footer: when the component last changed.
    changed_on(date: &str) { en: "changed on {date}", pt: "mudou em {date}",
        es: "cambió el {date}", fr: "modifié le {date}", de: "geändert am {date}",
        it: "modificato il {date}", ja: "{date}に変更", zh: "变更于 {date}",
        ko: "{date}에 변경됨", ru: "изменено {date}" }
    /// Block: how many more parts than the ones listed.
    more_parts(count: usize) { en: "+{count} parts", pt: "+{count} partes",
        es: "+{count} partes", fr: "+{count} parties", de: "+{count} Teile",
        it: "+{count} parti", ja: "+{count}パート", zh: "+{count} 个部分",
        ko: "+{count}개 파트", ru: "+{count} частей" }
}

// ---- timeline --------------------------------------------------------------

strings! {
    /// Timeline: nothing happened yet.
    nothing_happened { en: "Nothing has happened here yet.",
        pt: "Nada aconteceu aqui ainda.", es: "Aquí aún no ha pasado nada.",
        fr: "Rien ne s’est encore passé ici.", de: "Hier ist noch nichts passiert.",
        it: "Qui non è ancora successo nulla.", ja: "ここではまだ何も起きていません。",
        zh: "这里还没有发生任何事。", ko: "여기서는 아직 아무 일도 없었어요.",
        ru: "Здесь пока ничего не происходило." }
    /// Timeline day heading when only map upkeep happened.
    map_upkeep { en: "map upkeep", pt: "manutenção do mapa", es: "mantenimiento del mapa",
        fr: "entretien de la carte", de: "Kartenpflege", it: "manutenzione della mappa",
        ja: "マップの保守", zh: "地图维护", ko: "맵 유지 관리", ru: "обслуживание карты" }
    /// Timeline event: a decision was confirmed.
    timeline_decision_confirmed { en: "Decision confirmed", pt: "Decisão confirmada",
        es: "Decisión confirmada", fr: "Décision confirmée", de: "Entscheidung bestätigt",
        it: "Decisione confermata", ja: "決定を確定", zh: "决策已确认",
        ko: "결정 확인됨", ru: "Решение подтверждено" }
    /// Timeline event: a decision was superseded.
    timeline_decision_superseded { en: "Decision superseded", pt: "Decisão substituída",
        es: "Decisión sustituida", fr: "Décision remplacée", de: "Entscheidung ersetzt",
        it: "Decisione sostituita", ja: "決定を置き換え", zh: "决策已被取代",
        ko: "결정 대체됨", ru: "Решение заменено" }
    /// Timeline event: a rule took effect.
    timeline_rule_started { en: "Rule took effect", pt: "Regra passou a valer",
        es: "La regla entró en vigor", fr: "La règle est entrée en vigueur",
        de: "Regel gilt jetzt", it: "La regola è entrata in vigore",
        ja: "ルールが有効になりました", zh: "规则开始生效", ko: "규칙이 유효해짐",
        ru: "Правило вступило в силу" }
    /// Timeline event: a rule stopped holding.
    timeline_rule_ended { en: "Rule no longer in effect", pt: "Regra deixou de valer",
        es: "La regla dejó de estar vigente", fr: "La règle n’est plus en vigueur",
        de: "Regel gilt nicht mehr", it: "La regola non è più in vigore",
        ja: "ルールが無効になりました", zh: "规则不再生效", ko: "규칙이 효력을 잃음",
        ru: "Правило перестало действовать" }
    /// Timeline event: an item entered the map.
    timeline_entered_map { en: "Added to the map", pt: "Entrou no mapa",
        es: "Entró en el mapa", fr: "Entré dans la carte", de: "In die Karte aufgenommen",
        it: "Entrato nella mappa", ja: "マップに追加", zh: "已加入地图",
        ko: "맵에 추가됨", ru: "Добавлено на карту" }
    /// Timeline event: an item was retired.
    timeline_retired { en: "Retired", pt: "Aposentado", es: "Retirado", fr: "Retiré",
        de: "Stillgelegt", it: "Ritirato", ja: "廃止済み", zh: "已停用", ko: "폐기됨",
        ru: "Списано" }
    /// Timeline event: links were confirmed.
    timeline_links_confirmed { en: "Links confirmed", pt: "Ligações confirmadas",
        es: "Vínculos confirmados", fr: "Liens confirmés", de: "Verknüpfungen bestätigt",
        it: "Collegamenti confermati", ja: "リンクを確定", zh: "关联已确认",
        ko: "연결 확인됨", ru: "Ссылки подтверждены" }
    /// Timeline event: links were undone.
    timeline_links_undone { en: "Links undone", pt: "Ligações desfeitas",
        es: "Vínculos deshechos", fr: "Liens défaits", de: "Verknüpfungen gelöst",
        it: "Collegamenti sciolti", ja: "リンクを解除", zh: "关联已解除",
        ko: "연결 해제됨", ru: "Ссылки разорваны" }
}

formats! {
    /// Timeline: the names of a condensed run and how many more there are.
    upkeep_more(shown: &str, rest: usize) { en: "{shown} and {rest} more",
        pt: "{shown} e mais {rest}", es: "{shown} y {rest} más",
        fr: "{shown} et {rest} de plus", de: "{shown} und {rest} weitere",
        it: "{shown} e altri {rest}", ja: "{shown} ほか{rest}件",
        zh: "{shown} 等另外 {rest} 项", ko: "{shown} 외 {rest}개",
        ru: "{shown} и ещё {rest}" }
    /// Timeline: which decision replaced this one.
    superseded_by(label: &str) { en: "Superseded by: {label}",
        pt: "Substituída por: {label}", es: "Sustituida por: {label}",
        fr: "Remplacée par : {label}", de: "Ersetzt durch: {label}",
        it: "Sostituita da: {label}", ja: "置き換え先: {label}",
        zh: "取代者:{label}", ko: "대체한 결정: {label}", ru: "Заменено на: {label}" }
}

plurals! {
    /// Count of decisions.
    decisions_count(count: usize) {
        en: ["{count} decision", "{count} decisions"],
        pt: ["{count} decisão", "{count} decisões"],
        es: ["{count} decisión", "{count} decisiones"],
        fr: ["{count} décision", "{count} décisions"],
        de: ["{count} Entscheidung", "{count} Entscheidungen"],
        it: ["{count} decisione", "{count} decisioni"],
        ja: "決定{count}件", zh: "{count} 项决策", ko: "결정 {count}개",
        ru: ["{count} решение", "{count} решения", "{count} решений"]
    }
    /// Count of rules.
    rules_count(count: usize) {
        en: ["{count} rule", "{count} rules"],
        pt: ["{count} regra", "{count} regras"],
        es: ["{count} regla", "{count} reglas"],
        fr: ["{count} règle", "{count} règles"],
        de: ["{count} Regel", "{count} Regeln"],
        it: ["{count} regola", "{count} regole"],
        ja: "ルール{count}件", zh: "{count} 条规则", ko: "규칙 {count}개",
        ru: ["{count} правило", "{count} правила", "{count} правил"]
    }
    /// Count of conflicts.
    conflicts_count(count: usize) {
        en: ["{count} conflict", "{count} conflicts"],
        pt: ["{count} conflito", "{count} conflitos"],
        es: ["{count} conflicto", "{count} conflictos"],
        fr: ["{count} conflit", "{count} conflits"],
        de: ["{count} Konflikt", "{count} Konflikte"],
        it: ["{count} conflitto", "{count} conflitti"],
        ja: "競合{count}件", zh: "{count} 处冲突", ko: "충돌 {count}개",
        ru: ["{count} конфликт", "{count} конфликта", "{count} конфликтов"]
    }
    /// Timeline day heading: how many things changed.
    changes_count(count: usize) {
        en: ["{count} change", "{count} changes"],
        pt: ["{count} mudança", "{count} mudanças"],
        es: ["{count} cambio", "{count} cambios"],
        fr: ["{count} modification", "{count} modifications"],
        de: ["{count} Änderung", "{count} Änderungen"],
        it: ["{count} modifica", "{count} modifiche"],
        ja: "{count}件の変更", zh: "{count} 项变更", ko: "변경 {count}건",
        ru: ["{count} изменение", "{count} изменения", "{count} изменений"]
    }
    /// Headline of a condensed run of events of any other kind.
    events_count(count: usize) {
        en: ["{count} event", "{count} events"],
        pt: ["{count} evento", "{count} eventos"],
        es: ["{count} evento", "{count} eventos"],
        fr: ["{count} événement", "{count} événements"],
        de: ["{count} Ereignis", "{count} Ereignisse"],
        it: ["{count} evento", "{count} eventi"],
        ja: "{count}件のイベント", zh: "{count} 个事件", ko: "이벤트 {count}건",
        ru: ["{count} событие", "{count} события", "{count} событий"]
    }
    /// Headline of a condensed run: items that entered the map.
    items_entered(count: usize) {
        en: ["{count} item entered the map", "{count} items entered the map"],
        pt: ["{count} item entrou no mapa", "{count} itens entraram no mapa"],
        es: ["{count} elemento entró en el mapa", "{count} elementos entraron en el mapa"],
        fr: ["{count} élément est entré dans la carte",
            "{count} éléments sont entrés dans la carte"],
        de: ["{count} Element wurde in die Karte aufgenommen",
            "{count} Elemente wurden in die Karte aufgenommen"],
        it: ["{count} elemento è entrato nella mappa",
            "{count} elementi sono entrati nella mappa"],
        ja: "{count}件の項目がマップに追加されました",
        zh: "{count} 个条目加入了地图", ko: "항목 {count}개가 맵에 추가됐어요",
        ru: ["{count} элемент добавлен на карту", "{count} элемента добавлено на карту",
            "{count} элементов добавлено на карту"]
    }
    /// Headline of a condensed run: items that were retired.
    items_retired(count: usize) {
        en: ["{count} item retired", "{count} items retired"],
        pt: ["{count} item aposentado", "{count} itens aposentados"],
        es: ["{count} elemento retirado", "{count} elementos retirados"],
        fr: ["{count} élément retiré", "{count} éléments retirés"],
        de: ["{count} Element stillgelegt", "{count} Elemente stillgelegt"],
        it: ["{count} elemento ritirato", "{count} elementi ritirati"],
        ja: "{count}件の項目を廃止", zh: "{count} 个条目已停用",
        ko: "항목 {count}개 폐기됨",
        ru: ["{count} элемент списан", "{count} элемента списано",
            "{count} элементов списано"]
    }
    /// Headline of a condensed run: links that were confirmed.
    links_confirmed_count(count: usize) {
        en: ["{count} link confirmed", "{count} links confirmed"],
        pt: ["{count} ligação confirmada", "{count} ligações confirmadas"],
        es: ["{count} vínculo confirmado", "{count} vínculos confirmados"],
        fr: ["{count} lien confirmé", "{count} liens confirmés"],
        de: ["{count} Verknüpfung bestätigt", "{count} Verknüpfungen bestätigt"],
        it: ["{count} collegamento confermato", "{count} collegamenti confermati"],
        ja: "{count}件のリンクを確定", zh: "已确认 {count} 个关联",
        ko: "연결 {count}개 확인됨",
        ru: ["{count} ссылка подтверждена", "{count} ссылки подтверждены",
            "{count} ссылок подтверждено"]
    }
    /// Headline of a condensed run: links that were undone.
    links_undone_count(count: usize) {
        en: ["{count} link undone", "{count} links undone"],
        pt: ["{count} ligação desfeita", "{count} ligações desfeitas"],
        es: ["{count} vínculo deshecho", "{count} vínculos deshechos"],
        fr: ["{count} lien défait", "{count} liens défaits"],
        de: ["{count} Verknüpfung gelöst", "{count} Verknüpfungen gelöst"],
        it: ["{count} collegamento sciolto", "{count} collegamenti sciolti"],
        ja: "{count}件のリンクを解除", zh: "已解除 {count} 个关联",
        ko: "연결 {count}개 해제됨",
        ru: ["{count} ссылка разорвана", "{count} ссылки разорваны",
            "{count} ссылок разорвано"]
    }
}

// ---- what a suggestion says ------------------------------------------------

formats! {
    /// Sentence: the first decision depends on the second.
    relation_depends(from: &str, to: &str) {
        en: "The decision {BOLD}“{from}”{BOLD} depends on the decision {BOLD}“{to}”{BOLD} \
             and only makes sense because the second was made.",
        pt: "A decisão {BOLD}“{from}”{BOLD} depende da decisão {BOLD}“{to}”{BOLD} só faz \
             sentido porque a segunda foi tomada.",
        es: "La decisión {BOLD}“{from}”{BOLD} depende de la decisión {BOLD}“{to}”{BOLD} y \
             solo tiene sentido porque la segunda se tomó.",
        fr: "La décision {BOLD}“{from}”{BOLD} dépend de la décision {BOLD}“{to}”{BOLD} et \
             n’a de sens que parce que la seconde a été prise.",
        de: "Die Entscheidung {BOLD}„{from}“{BOLD} hängt von der Entscheidung \
             {BOLD}„{to}“{BOLD} ab und ergibt nur Sinn, weil die zweite getroffen wurde.",
        it: "La decisione {BOLD}“{from}”{BOLD} dipende dalla decisione {BOLD}“{to}”{BOLD} e \
             ha senso solo perché la seconda è stata presa.",
        ja: "決定 {BOLD}「{from}」{BOLD} は決定 {BOLD}「{to}」{BOLD} に依存しており、 \
             後者が決まったからこそ 意味を持ちます。",
        zh: "决策 {BOLD}“{from}”{BOLD} 依赖于决策 {BOLD}“{to}”{BOLD}， \
             只有因为后者已被采纳 才有意义。",
        ko: "{BOLD}“{from}”{BOLD} 결정은 {BOLD}“{to}”{BOLD} 결정에 의존하며, 두 번째 결정이 \
             내려졌기 때문에 의미가 있어요.",
        ru: "Решение {BOLD}«{from}»{BOLD} зависит от решения {BOLD}«{to}»{BOLD} и имеет \
             смысл только потому, что второе было принято." }
    /// Sentence: the first decision seems to contradict the second.
    relation_conflicts(from: &str, to: &str) {
        en: "The decision {BOLD}“{from}”{BOLD} seems to contradict the decision \
             {BOLD}“{to}”{BOLD}.",
        pt: "A decisão {BOLD}“{from}”{BOLD} parece contradizer a decisão {BOLD}“{to}”{BOLD}.",
        es: "La decisión {BOLD}“{from}”{BOLD} parece contradecir la decisión \
             {BOLD}“{to}”{BOLD}.",
        fr: "La décision {BOLD}“{from}”{BOLD} semble contredire la décision \
             {BOLD}“{to}”{BOLD}.",
        de: "Die Entscheidung {BOLD}„{from}“{BOLD} scheint der Entscheidung \
             {BOLD}„{to}“{BOLD} zu widersprechen.",
        it: "La decisione {BOLD}“{from}”{BOLD} sembra contraddire la decisione \
             {BOLD}“{to}”{BOLD}.",
        ja: "決定 {BOLD}「{from}」{BOLD} は決定 {BOLD}「{to}」{BOLD} と矛盾しているようです。",
        zh: "决策 {BOLD}“{from}”{BOLD} 似乎与决策 {BOLD}“{to}”{BOLD} 矛盾。",
        ko: "{BOLD}“{from}”{BOLD} 결정이 {BOLD}“{to}”{BOLD} 결정과 모순되는 것 같아요.",
        ru: "Решение {BOLD}«{from}»{BOLD}, похоже, противоречит решению {BOLD}«{to}»{BOLD}." }
    /// Sentence: the first decision seems to have replaced the second.
    relation_supersedes_sentence(from: &str, to: &str) {
        en: "The decision {BOLD}“{from}”{BOLD} seems to have replaced the decision \
             {BOLD}“{to}”{BOLD}.",
        pt: "A decisão {BOLD}“{from}”{BOLD} parece ter substituído a decisão \
             {BOLD}“{to}”{BOLD}.",
        es: "La decisión {BOLD}“{from}”{BOLD} parece haber sustituido a la decisión \
             {BOLD}“{to}”{BOLD}.",
        fr: "La décision {BOLD}“{from}”{BOLD} semble avoir remplacé la décision \
             {BOLD}“{to}”{BOLD}.",
        de: "Die Entscheidung {BOLD}„{from}“{BOLD} scheint die Entscheidung \
             {BOLD}„{to}“{BOLD} ersetzt zu haben.",
        it: "La decisione {BOLD}“{from}”{BOLD} sembra aver sostituito la decisione \
             {BOLD}“{to}”{BOLD}.",
        ja: "決定 {BOLD}「{from}」{BOLD} は決定 {BOLD}「{to}」{BOLD} を置き換えたようです。",
        zh: "决策 {BOLD}“{from}”{BOLD} 似乎取代了决策 {BOLD}“{to}”{BOLD}。",
        ko: "{BOLD}“{from}”{BOLD} 결정이 {BOLD}“{to}”{BOLD} 결정을 대체한 것 같아요.",
        ru: "Решение {BOLD}«{from}»{BOLD}, похоже, заменило решение {BOLD}«{to}»{BOLD}." }
    /// Sentence: a rule mentions a map item in its own text.
    link_mention_rule(source: &str, target: &str, quote: &str) {
        en: "The rule {BOLD}“{source}”{BOLD} mentions {BOLD}“{target}”{BOLD} in its own text: \
             {BOLD}“{quote}”{BOLD}.",
        pt: "A regra {BOLD}“{source}”{BOLD} cita {BOLD}“{target}”{BOLD} no próprio texto: \
             {BOLD}“{quote}”{BOLD}.",
        es: "La regla {BOLD}“{source}”{BOLD} cita {BOLD}“{target}”{BOLD} en su propio texto: \
             {BOLD}“{quote}”{BOLD}.",
        fr: "La règle {BOLD}“{source}”{BOLD} cite {BOLD}“{target}”{BOLD} dans son propre \
             texte : {BOLD}“{quote}”{BOLD}.",
        de: "Die Regel {BOLD}„{source}“{BOLD} nennt {BOLD}„{target}“{BOLD} in ihrem eigenen \
             Text: {BOLD}„{quote}“{BOLD}.",
        it: "La regola {BOLD}“{source}”{BOLD} cita {BOLD}“{target}”{BOLD} nel proprio testo: \
             {BOLD}“{quote}”{BOLD}.",
        ja: "ルール {BOLD}「{source}」{BOLD} は、自身の文中で {BOLD}「{target}」{BOLD} に言及して\
             います: {BOLD}「{quote}」{BOLD}。",
        zh: "规则 {BOLD}“{source}”{BOLD} 在自身文字中提到了 {BOLD}“{target}”{BOLD}： \
             {BOLD}“{quote}”{BOLD}。",
        ko: "{BOLD}“{source}”{BOLD} 규칙이 본문에서 {BOLD}“{target}”{BOLD}을(를) 언급해요: \
             {BOLD}“{quote}”{BOLD}.",
        ru: "Правило {BOLD}«{source}»{BOLD} упоминает {BOLD}«{target}»{BOLD} в собственном \
             тексте: {BOLD}«{quote}»{BOLD}." }
    /// Sentence: a decision mentions a map item in its own text.
    link_mention_decision(source: &str, target: &str, quote: &str) {
        en: "The decision {BOLD}“{source}”{BOLD} mentions {BOLD}“{target}”{BOLD} in its own \
             text: {BOLD}“{quote}”{BOLD}.",
        pt: "A decisão {BOLD}“{source}”{BOLD} cita {BOLD}“{target}”{BOLD} no próprio texto: \
             {BOLD}“{quote}”{BOLD}.",
        es: "La decisión {BOLD}“{source}”{BOLD} cita {BOLD}“{target}”{BOLD} en su propio \
             texto: {BOLD}“{quote}”{BOLD}.",
        fr: "La décision {BOLD}“{source}”{BOLD} cite {BOLD}“{target}”{BOLD} dans son propre \
             texte : {BOLD}“{quote}”{BOLD}.",
        de: "Die Entscheidung {BOLD}„{source}“{BOLD} nennt {BOLD}„{target}“{BOLD} in ihrem \
             eigenen Text: {BOLD}„{quote}“{BOLD}.",
        it: "La decisione {BOLD}“{source}”{BOLD} cita {BOLD}“{target}”{BOLD} nel proprio \
             testo: {BOLD}“{quote}”{BOLD}.",
        ja: "決定 {BOLD}「{source}」{BOLD} は、自身の文中で {BOLD}「{target}」{BOLD} に言及して\
             います: {BOLD}「{quote}」{BOLD}。",
        zh: "决策 {BOLD}“{source}”{BOLD} 在自身文字中提到了 {BOLD}“{target}”{BOLD}： \
             {BOLD}“{quote}”{BOLD}。",
        ko: "{BOLD}“{source}”{BOLD} 결정이 본문에서 {BOLD}“{target}”{BOLD}을(를) 언급해요: \
             {BOLD}“{quote}”{BOLD}.",
        ru: "Решение {BOLD}«{source}»{BOLD} упоминает {BOLD}«{target}»{BOLD} в собственном \
             тексте: {BOLD}«{quote}»{BOLD}." }
    /// Sentence: the AI proposed that a decision applies to a map item, with
    /// the passage of the decision and its own reason.
    link_ai_decision(source: &str, target: &str, quote: &str, why: &str) {
        en: "The AI proposes that the decision {BOLD}“{source}”{BOLD} applies to              {BOLD}“{target}”{BOLD}, based on this passage: {BOLD}“{quote}”{BOLD}. {why}",
        pt: "A IA propõe que a decisão {BOLD}“{source}”{BOLD} vale para              {BOLD}“{target}”{BOLD}, com base neste trecho: {BOLD}“{quote}”{BOLD}. {why}",
        es: "La IA propone que la decisión {BOLD}“{source}”{BOLD} se aplica a              {BOLD}“{target}”{BOLD}, según este fragmento: {BOLD}“{quote}”{BOLD}. {why}",
        fr: "L’IA propose que la décision {BOLD}“{source}”{BOLD} s’applique à              {BOLD}“{target}”{BOLD}, d’après cet extrait : {BOLD}“{quote}”{BOLD}. {why}",
        de: "Die KI schlägt vor, dass die Entscheidung {BOLD}„{source}“{BOLD} für              {BOLD}„{target}“{BOLD} gilt, nach dieser Passage: {BOLD}„{quote}“{BOLD}. {why}",
        it: "L’IA propone che la decisione {BOLD}“{source}”{BOLD} valga per              {BOLD}“{target}”{BOLD}, in base a questo passaggio: {BOLD}“{quote}”{BOLD}. {why}",
        ja: "AI は、決定 {BOLD}「{source}」{BOLD} が {BOLD}「{target}」{BOLD} に適用されると提案             しています。根拠の一節: {BOLD}「{quote}」{BOLD}。{why}",
        zh: "AI 建议决策 {BOLD}“{source}”{BOLD} 适用于 {BOLD}“{target}”{BOLD}，依据这段文字：              {BOLD}“{quote}”{BOLD}。{why}",
        ko: "AI가 {BOLD}“{source}”{BOLD} 결정이 {BOLD}“{target}”{BOLD}에 적용된다고 제안해요.              근거 문장: {BOLD}“{quote}”{BOLD}. {why}",
        ru: "ИИ предлагает применить решение {BOLD}«{source}»{BOLD} к {BOLD}«{target}»{BOLD}              на основании этого фрагмента: {BOLD}«{quote}»{BOLD}. {why}" }
    /// Sentence: the AI proposed that a rule applies to a map item, with the
    /// passage of the rule and its own reason.
    link_ai_rule(source: &str, target: &str, quote: &str, why: &str) {
        en: "The AI proposes that the rule {BOLD}“{source}”{BOLD} applies to \
             {BOLD}“{target}”{BOLD}, based on this passage: {BOLD}“{quote}”{BOLD}. {why}",
        pt: "A IA propõe que a regra {BOLD}“{source}”{BOLD} vale para \
             {BOLD}“{target}”{BOLD}, com base neste trecho: {BOLD}“{quote}”{BOLD}. {why}",
        es: "La IA propone que la regla {BOLD}“{source}”{BOLD} se aplica a \
             {BOLD}“{target}”{BOLD}, según este fragmento: {BOLD}“{quote}”{BOLD}. {why}",
        fr: "L’IA propose que la règle {BOLD}“{source}”{BOLD} s’applique à \
             {BOLD}“{target}”{BOLD}, d’après cet extrait : {BOLD}“{quote}”{BOLD}. {why}",
        de: "Die KI schlägt vor, dass die Regel {BOLD}„{source}“{BOLD} für \
             {BOLD}„{target}“{BOLD} gilt, nach dieser Passage: {BOLD}„{quote}“{BOLD}. {why}",
        it: "L’IA propone che la regola {BOLD}“{source}”{BOLD} valga per \
             {BOLD}“{target}”{BOLD}, in base a questo passaggio: {BOLD}“{quote}”{BOLD}. {why}",
        ja: "AI は、ルール {BOLD}「{source}」{BOLD} が {BOLD}「{target}」{BOLD} に適用されると \
             提案しています。根拠の一節: {BOLD}「{quote}」{BOLD}。{why}",
        zh: "AI 建议规则 {BOLD}“{source}”{BOLD} 适用于 {BOLD}“{target}”{BOLD}， \
             依据这段文字：{BOLD}“{quote}”{BOLD}。{why}",
        ko: "AI가 {BOLD}“{source}”{BOLD} 규칙이 {BOLD}“{target}”{BOLD}에 적용된다고 \
             제안해요. 근거 문장: {BOLD}“{quote}”{BOLD}. {why}",
        ru: "ИИ предлагает применить правило {BOLD}«{source}»{BOLD} к {BOLD}«{target}»{BOLD} \
             на основании этого фрагмента: {BOLD}«{quote}»{BOLD}. {why}" }
    /// Sentence: a rule added a dependency that is a map technology.
    link_uses_rule(source: &str, reason: &str, target: &str) {
        en: "The rule {BOLD}“{source}”{BOLD} added the dependency {BOLD}{reason}{BOLD}, which \
             is the technology {BOLD}“{target}”{BOLD} on the Map.",
        pt: "A regra {BOLD}“{source}”{BOLD} adicionou a dependência {BOLD}{reason}{BOLD} que \
             é a tecnologia {BOLD}“{target}”{BOLD} do Mapa.",
        es: "La regla {BOLD}“{source}”{BOLD} añadió la dependencia {BOLD}{reason}{BOLD}, que \
             es la tecnología {BOLD}“{target}”{BOLD} del Mapa.",
        fr: "La règle {BOLD}“{source}”{BOLD} a ajouté la dépendance {BOLD}{reason}{BOLD}, qui \
             est la technologie {BOLD}“{target}”{BOLD} de la Carte.",
        de: "Die Regel {BOLD}„{source}“{BOLD} hat die Abhängigkeit {BOLD}{reason}{BOLD} \
             hinzugefügt, die der Technologie {BOLD}„{target}“{BOLD} der Karte entspricht.",
        it: "La regola {BOLD}“{source}”{BOLD} ha aggiunto la dipendenza {BOLD}{reason}{BOLD}, \
             che è la tecnologia {BOLD}“{target}”{BOLD} della Mappa.",
        ja: "ルール {BOLD}「{source}」{BOLD} が依存関係 {BOLD}{reason}{BOLD} を追加しました。 \
             これはマップのテクノロジー {BOLD}「{target}」{BOLD} です。",
        zh: "规则 {BOLD}“{source}”{BOLD} 添加了依赖 {BOLD}{reason}{BOLD}， \
             它就是地图中的技术 {BOLD}“{target}”{BOLD}。",
        ko: "{BOLD}“{source}”{BOLD} 규칙이 추가한 의존성: {BOLD}{reason}{BOLD}. 맵의 기술 \
             {BOLD}“{target}”{BOLD}에 해당해요.",
        ru: "Правило {BOLD}«{source}»{BOLD} добавило зависимость {BOLD}{reason}{BOLD} — это \
             технология {BOLD}«{target}»{BOLD} с карты." }
    /// Sentence: a decision added a dependency that is a map technology.
    link_uses_decision(source: &str, reason: &str, target: &str) {
        en: "The decision {BOLD}“{source}”{BOLD} added the dependency {BOLD}{reason}{BOLD}, \
             which is the technology {BOLD}“{target}”{BOLD} on the Map.",
        pt: "A decisão {BOLD}“{source}”{BOLD} adicionou a dependência {BOLD}{reason}{BOLD} \
             que é a tecnologia {BOLD}“{target}”{BOLD} do Mapa.",
        es: "La decisión {BOLD}“{source}”{BOLD} añadió la dependencia {BOLD}{reason}{BOLD}, \
             que es la tecnología {BOLD}“{target}”{BOLD} del Mapa.",
        fr: "La décision {BOLD}“{source}”{BOLD} a ajouté la dépendance {BOLD}{reason}{BOLD}, \
             qui est la technologie {BOLD}“{target}”{BOLD} de la Carte.",
        de: "Die Entscheidung {BOLD}„{source}“{BOLD} hat die Abhängigkeit {BOLD}{reason}{BOLD} \
             hinzugefügt, die der Technologie {BOLD}„{target}“{BOLD} der Karte entspricht.",
        it: "La decisione {BOLD}“{source}”{BOLD} ha aggiunto la dipendenza \
             {BOLD}{reason}{BOLD}, che è la tecnologia {BOLD}“{target}”{BOLD} della Mappa.",
        ja: "決定 {BOLD}「{source}」{BOLD} が依存関係 {BOLD}{reason}{BOLD} を追加しました。 \
             これはマップのテクノロジー {BOLD}「{target}」{BOLD} です。",
        zh: "决策 {BOLD}“{source}”{BOLD} 添加了依赖 {BOLD}{reason}{BOLD}， \
             它就是地图中的技术 {BOLD}“{target}”{BOLD}。",
        ko: "{BOLD}“{source}”{BOLD} 결정이 추가한 의존성: {BOLD}{reason}{BOLD}. 맵의 기술 \
             {BOLD}“{target}”{BOLD}에 해당해요.",
        ru: "Решение {BOLD}«{source}»{BOLD} добавило зависимость {BOLD}{reason}{BOLD} — это \
             технология {BOLD}«{target}»{BOLD} с карты." }
    /// Sentence: a rule applies to changes in a file of a component.
    link_applies(source: &str, reason: &str, target: &str) {
        en: "The rule {BOLD}“{source}”{BOLD} applies to changes in {BOLD}{reason}{BOLD}, a \
             file that belongs to the component {BOLD}“{target}”{BOLD}.",
        pt: "A regra {BOLD}“{source}”{BOLD} se aplica a mudanças em {BOLD}{reason}{BOLD}, \
             arquivo que pertence ao componente {BOLD}“{target}”{BOLD}.",
        es: "La regla {BOLD}“{source}”{BOLD} se aplica a cambios en {BOLD}{reason}{BOLD}, \
             archivo que pertenece al componente {BOLD}“{target}”{BOLD}.",
        fr: "La règle {BOLD}“{source}”{BOLD} s’applique aux modifications de \
             {BOLD}{reason}{BOLD}, fichier qui appartient au composant {BOLD}“{target}”{BOLD}.",
        de: "Die Regel {BOLD}„{source}“{BOLD} gilt für Änderungen an {BOLD}{reason}{BOLD}, \
             einer Datei, die zur Komponente {BOLD}„{target}“{BOLD} gehört.",
        it: "La regola {BOLD}“{source}”{BOLD} si applica alle modifiche in \
             {BOLD}{reason}{BOLD}, file che appartiene al componente {BOLD}“{target}”{BOLD}.",
        ja: "ルール {BOLD}「{source}」{BOLD} は、コンポーネント {BOLD}「{target}」{BOLD} に属する\
             ファイル {BOLD}{reason}{BOLD} への変更に適用されます。",
        zh: "规则 {BOLD}“{source}”{BOLD} 适用于对 {BOLD}{reason}{BOLD} 的更改， \
             该文件属于组件 {BOLD}“{target}”{BOLD}。",
        ko: "{BOLD}“{source}”{BOLD} 규칙은 컴포넌트 {BOLD}“{target}”{BOLD}에 속한 파일 \
             {BOLD}{reason}{BOLD}의 변경에 적용돼요.",
        ru: "Правило {BOLD}«{source}»{BOLD} применяется к изменениям в {BOLD}{reason}{BOLD} — \
             файле, принадлежащем компоненту {BOLD}«{target}»{BOLD}." }
    /// Sentence: a decision touched a file of a component.
    link_touched(source: &str, reason: &str, target: &str) {
        en: "The decision {BOLD}“{source}”{BOLD} touched {BOLD}{reason}{BOLD}, a file that \
             belongs to the component {BOLD}“{target}”{BOLD}.",
        pt: "A decisão {BOLD}“{source}”{BOLD} mexeu em {BOLD}{reason}{BOLD}, arquivo que \
             pertence ao componente {BOLD}“{target}”{BOLD}.",
        es: "La decisión {BOLD}“{source}”{BOLD} tocó {BOLD}{reason}{BOLD}, archivo que \
             pertenece al componente {BOLD}“{target}”{BOLD}.",
        fr: "La décision {BOLD}“{source}”{BOLD} a touché {BOLD}{reason}{BOLD}, fichier qui \
             appartient au composant {BOLD}“{target}”{BOLD}.",
        de: "Die Entscheidung {BOLD}„{source}“{BOLD} hat {BOLD}{reason}{BOLD} berührt, eine \
             Datei, die zur Komponente {BOLD}„{target}“{BOLD} gehört.",
        it: "La decisione {BOLD}“{source}”{BOLD} ha toccato {BOLD}{reason}{BOLD}, file che \
             appartiene al componente {BOLD}“{target}”{BOLD}.",
        ja: "決定 {BOLD}「{source}」{BOLD} は、コンポーネント {BOLD}「{target}」{BOLD} に属する\
             ファイル {BOLD}{reason}{BOLD} を変更しました。",
        zh: "决策 {BOLD}“{source}”{BOLD} 修改了 {BOLD}{reason}{BOLD}， \
             该文件属于组件 {BOLD}“{target}”{BOLD}。",
        ko: "{BOLD}“{source}”{BOLD} 결정이 건드린 파일: {BOLD}{reason}{BOLD} — 컴포넌트 \
             {BOLD}“{target}”{BOLD}에 속한 파일이에요.",
        ru: "Решение {BOLD}«{source}»{BOLD} затронуло {BOLD}{reason}{BOLD} — файл, \
             принадлежащий компоненту {BOLD}«{target}»{BOLD}." }
    /// Sentence: a component the workspace declares is not on the map yet.
    item_declared(workspace: &str, name: &str, pattern: &str) {
        en: "In {workspace}, the part {BOLD}“{name}”{BOLD} ({BOLD}{pattern}{BOLD}) is \
             declared, but it isn’t on the Map yet.",
        pt: "O {workspace} declara a parte {BOLD}“{name}”{BOLD} ({BOLD}{pattern}{BOLD}), mas \
             ela ainda não está no Mapa.",
        es: "En {workspace} se declara la parte {BOLD}“{name}”{BOLD} ({BOLD}{pattern}{BOLD}), \
             pero aún no está en el Mapa.",
        fr: "Dans {workspace}, la partie {BOLD}“{name}”{BOLD} ({BOLD}{pattern}{BOLD}) est \
             déclarée, mais elle n’est pas encore dans la Carte.",
        de: "In {workspace} ist der Teil {BOLD}„{name}“{BOLD} ({BOLD}{pattern}{BOLD}) \
             deklariert, aber er ist noch nicht in der Karte.",
        it: "In {workspace} è dichiarata la parte {BOLD}“{name}”{BOLD} \
             ({BOLD}{pattern}{BOLD}), ma non è ancora nella Mappa.",
        ja: "{workspace} でパート {BOLD}「{name}」{BOLD} ({BOLD}{pattern}{BOLD}) が宣言されて\
             いますが、 まだマップにはありません。",
        zh: "{workspace} 中声明了部分 {BOLD}“{name}”{BOLD} ({BOLD}{pattern}{BOLD})， \
             但它还不在地图中。",
        ko: "{workspace}에 선언된 파트: {BOLD}“{name}”{BOLD} ({BOLD}{pattern}{BOLD}). 아직 \
             맵에 없어요.",
        ru: "В {workspace} объявлена часть {BOLD}«{name}»{BOLD} ({BOLD}{pattern}{BOLD}), но \
             её ещё нет на карте." }
    /// What creating a declared component changes.
    item_declared_effect() {
        en: "the component enters the Map and decisions that touched those files can be \
             linked to it.",
        pt: "o componente entra no Mapa e as decisões que mexeram nesses arquivos passam a \
             poder ser ligadas a ele.",
        es: "el componente entra en el Mapa y las decisiones que tocaron esos archivos pasan \
             a poder vincularse a él.",
        fr: "le composant entre dans la Carte et les décisions qui ont touché ces fichiers \
             peuvent lui être liées.",
        de: "die Komponente kommt in die Karte, und Entscheidungen, die diese Dateien \
             berührt haben, lassen sich mit ihr verknüpfen.",
        it: "il componente entra nella Mappa e le decisioni che hanno toccato quei file \
             possono essere collegate a esso.",
        ja: "コンポーネントがマップに追加され、それらのファイルに触れた決定をリンクできる\
             ようになります。",
        zh: "该组件将加入地图,之前涉及这些文件的决策就可以与它关联。",
        ko: "컴포넌트가 맵에 들어가고, 그 파일을 건드린 결정을 연결할 수 있게 돼요.",
        ru: "компонент появится на карте, и решения, затронувшие эти файлы, можно будет с \
             ним связать." }
    /// What creating a technology changes.
    item_technology_effect(name: &str) {
        en: "{name} enters the Map as a technology and the decisions that use it become \
             linked to it.",
        pt: "{name} entra no Mapa como tecnologia e as decisões que a usam passam a estar \
             ligadas a ela.",
        es: "{name} entra en el Mapa como tecnología y las decisiones que la usan pasan a \
             estar vinculadas a ella.",
        fr: "{name} entre dans la Carte comme technologie et les décisions qui l’utilisent \
             lui sont liées.",
        de: "{name} kommt als Technologie in die Karte, und die Entscheidungen, die sie \
             verwenden, werden mit ihr verknüpft.",
        it: "{name} entra nella Mappa come tecnologia e le decisioni che la usano vengono \
             collegate a essa.",
        ja: "{name}がテクノロジーとしてマップに追加され、それを使う決定がリンクされます。",
        zh: "{name} 将作为技术加入地图,使用它的决策会与之关联。",
        ko: "{name}: 기술로 맵에 들어가고, 이를 사용하는 결정이 연결돼요.",
        ru: "{name} появится на карте как технология, и использующие её решения будут с ней \
             связаны." }
    /// What confirming a link between a rule and a map technology changes.
    link_uses_effect(target: &str) {
        en: "{target} will be listed as a technology used by this decision, on its page and \
             on the Map’s.",
        pt: "{target} passa a constar como tecnologia usada por essa decisão, na página dela \
             e na do Mapa.",
        es: "{target} pasa a figurar como tecnología usada por esta decisión, en su página y \
             en la del Mapa.",
        fr: "{target} figurera comme technologie utilisée par cette décision, sur sa page et \
             sur celle de la Carte.",
        de: "{target} wird als von dieser Entscheidung verwendete Technologie geführt, auf \
             ihrer Seite und auf der Seite der Karte.",
        it: "{target} risulterà come tecnologia usata da questa decisione, nella sua pagina e \
             in quella della Mappa.",
        ja: "{target}は、この決定が使うテクノロジーとして、決定のページとマップに載ります。",
        zh: "{target} 将作为该决策所使用的技术,显示在决策页面和地图上。",
        ko: "{target}이(가) 이 결정이 쓰는 기술로 결정 페이지와 맵에 표시돼요.",
        ru: "{target} будет значиться как технология, используемая этим решением, на её \
             странице и на карте." }
    /// What confirming a rule that applies to a component's files changes.
    link_applies_effect(target: &str) {
        en: "the rule starts applying to {target} and is handed to the agent when it edits \
             files of that component.",
        pt: "a regra passa a valer em {target} e é entregue ao agente quando ele edita \
             arquivos desse componente.",
        es: "la regla pasa a valer en {target} y se entrega al agente cuando edita archivos \
             de ese componente.",
        fr: "la règle s’applique désormais à {target} et est transmise à l’agent quand il \
             modifie des fichiers de ce composant.",
        de: "die Regel gilt dann für {target} und wird dem Agent übergeben, wenn er Dateien \
             dieser Komponente bearbeitet.",
        it: "la regola inizia a valere per {target} e viene consegnata all’agente quando \
             modifica file di quel componente.",
        ja: "ルールが{target}で有効になり、エージェントがそのコンポーネントのファイルを\
             編集するときに渡されます。",
        zh: "该规则将在 {target} 中生效,并在智能体编辑该组件的文件时交给它。",
        ko: "규칙이 {target}에 적용되고, 에이전트가 그 컴포넌트의 파일을 편집할 때 전달돼요.",
        ru: "правило начнёт действовать в {target} и будет передаваться агенту, когда он \
             редактирует файлы этого компонента." }
    /// What confirming a decision that touched a component's files changes.
    link_touched_effect(target: &str) {
        en: "the decision starts applying to {target}: it shows on the component’s page and \
             is handed to the agent when it edits its files.",
        pt: "a decisão passa a valer para {target}: aparece na página do componente e é \
             entregue ao agente quando ele edita arquivos dele.",
        es: "la decisión pasa a valer para {target}: aparece en la página del componente y se \
             entrega al agente cuando edita sus archivos.",
        fr: "la décision s’applique désormais à {target} : elle apparaît sur la page du \
             composant et est transmise à l’agent quand il en modifie les fichiers.",
        de: "die Entscheidung gilt dann für {target}: Sie erscheint auf der Seite der \
             Komponente und wird dem Agent übergeben, wenn er deren Dateien bearbeitet.",
        it: "la decisione inizia a valere per {target}: compare nella pagina del componente e \
             viene consegnata all’agente quando ne modifica i file.",
        ja: "決定が{target}で有効になり、コンポーネントのページに表示され、エージェントが\
             そのファイルを編集するときに渡されます。",
        zh: "该决策将对 {target} 生效:它会出现在组件页面上,并在智能体编辑该组件的文件时交给它。",
        ko: "결정이 {target}에 적용돼요. 컴포넌트 페이지에 나타나고, 에이전트가 그 파일을 \
             편집할 때 전달돼요.",
        ru: "решение начнёт действовать для {target}: оно появится на странице компонента и \
             будет передаваться агенту, когда он редактирует его файлы." }
    /// Effect of a mention of a technology by a rule.
    link_mention_uses_rule_effect(target: &str) {
        en: "{target} will be listed as a technology used by this rule on the Map. The link \
             came only from the text; check the passage before confirming.",
        pt: "{target} passa a constar como tecnologia usada por essa regra no Mapa. A ligação \
             veio só do texto; confira o trecho antes de confirmar.",
        es: "{target} pasa a figurar como tecnología usada por esta regla en el Mapa. El \
             vínculo vino solo del texto; revisa el fragmento antes de confirmar.",
        fr: "{target} figurera comme technologie utilisée par cette règle dans la Carte. Le \
             lien vient uniquement du texte ; vérifiez l’extrait avant de confirmer.",
        de: "{target} wird in der Karte als von dieser Regel verwendete Technologie geführt. \
             Die Verknüpfung stammt nur aus dem Text; prüfe die Textstelle vor dem Bestätigen.",
        it: "{target} risulterà come tecnologia usata da questa regola nella Mappa. Il \
             collegamento deriva solo dal testo; controlla il passaggio prima di confermare.",
        ja: "{target}は、このルールが使うテクノロジーとしてマップに載ります。このリンクは文章\
             だけから見つかったものなので、確定する前に該当箇所を確認してください。",
        zh: "{target} 将在地图中显示为该规则所使用的技术。这条关联仅来自文字,确认前请检查相关片段。",
        ko: "{target}이(가) 이 규칙이 쓰는 기술로 맵에 표시돼요. 이 연결은 텍스트에서만 \
             나온 것이니, 확인하기 전에 해당 구절을 살펴보세요.",
        ru: "{target} будет значиться на карте как технология, используемая этим правилом. \
             Связь найдена только по тексту; проверьте фрагмент перед подтверждением." }
    /// Effect of a mention of a technology by a decision.
    link_mention_uses_decision_effect(target: &str) {
        en: "{target} will be listed as a technology used by this decision on the Map. The \
             link came only from the text; check the passage before confirming.",
        pt: "{target} passa a constar como tecnologia usada por essa decisão no Mapa. A \
             ligação veio só do texto; confira o trecho antes de confirmar.",
        es: "{target} pasa a figurar como tecnología usada por esta decisión en el Mapa. El \
             vínculo vino solo del texto; revisa el fragmento antes de confirmar.",
        fr: "{target} figurera comme technologie utilisée par cette décision dans la Carte. \
             Le lien vient uniquement du texte ; vérifiez l’extrait avant de confirmer.",
        de: "{target} wird in der Karte als von dieser Entscheidung verwendete Technologie \
             geführt. Die Verknüpfung stammt nur aus dem Text; prüfe die Textstelle vor dem \
             Bestätigen.",
        it: "{target} risulterà come tecnologia usata da questa decisione nella Mappa. Il \
             collegamento deriva solo dal testo; controlla il passaggio prima di confermare.",
        ja: "{target}は、この決定が使うテクノロジーとしてマップに載ります。このリンクは文章\
             だけから見つかったものなので、確定する前に該当箇所を確認してください。",
        zh: "{target} 将在地图中显示为该决策所使用的技术。这条关联仅来自文字,确认前请检查相关片段。",
        ko: "{target}이(가) 이 결정이 쓰는 기술로 맵에 표시돼요. 이 연결은 텍스트에서만 \
             나온 것이니, 확인하기 전에 해당 구절을 살펴보세요.",
        ru: "{target} будет значиться на карте как технология, используемая этим решением. \
             Связь найдена только по тексту; проверьте фрагмент перед подтверждением." }
    /// Effect of a mention of a component by a rule.
    link_mention_affects_rule_effect(target: &str) {
        en: "{target} will be listed as a part affected by this rule on the Map. The link \
             came only from the text; check the passage before confirming.",
        pt: "{target} passa a constar como parte afetada por essa regra no Mapa. A ligação \
             veio só do texto; confira o trecho antes de confirmar.",
        es: "{target} pasa a figurar como parte afectada por esta regla en el Mapa. El \
             vínculo vino solo del texto; revisa el fragmento antes de confirmar.",
        fr: "{target} figurera comme partie concernée par cette règle dans la Carte. Le lien \
             vient uniquement du texte ; vérifiez l’extrait avant de confirmer.",
        de: "{target} wird in der Karte als von dieser Regel betroffener Teil geführt. Die \
             Verknüpfung stammt nur aus dem Text; prüfe die Textstelle vor dem Bestätigen.",
        it: "{target} risulterà come parte interessata da questa regola nella Mappa. Il \
             collegamento deriva solo dal testo; controlla il passaggio prima di confermare.",
        ja: "{target}は、このルールの影響を受ける部分としてマップに載ります。このリンクは文章\
             だけから見つかったものなので、確定する前に該当箇所を確認してください。",
        zh: "{target} 将在地图中显示为受该规则影响的部分。这条关联仅来自文字,确认前请检查相关片段。",
        ko: "{target}이(가) 이 규칙의 영향을 받는 부분으로 맵에 표시돼요. 이 연결은 \
             텍스트에서만 나온 것이니, 확인하기 전에 해당 구절을 살펴보세요.",
        ru: "{target} будет значиться на карте как часть, затронутая этим правилом. Связь \
             найдена только по тексту; проверьте фрагмент перед подтверждением." }
    /// Effect of a mention of a component by a decision.
    link_mention_affects_decision_effect(target: &str) {
        en: "{target} will be listed as a part affected by this decision on the Map. The \
             link came only from the text; check the passage before confirming.",
        pt: "{target} passa a constar como parte afetada por essa decisão no Mapa. A ligação \
             veio só do texto; confira o trecho antes de confirmar.",
        es: "{target} pasa a figurar como parte afectada por esta decisión en el Mapa. El \
             vínculo vino solo del texto; revisa el fragmento antes de confirmar.",
        fr: "{target} figurera comme partie concernée par cette décision dans la Carte. Le \
             lien vient uniquement du texte ; vérifiez l’extrait avant de confirmer.",
        de: "{target} wird in der Karte als von dieser Entscheidung betroffener Teil geführt. \
             Die Verknüpfung stammt nur aus dem Text; prüfe die Textstelle vor dem Bestätigen.",
        it: "{target} risulterà come parte interessata da questa decisione nella Mappa. Il \
             collegamento deriva solo dal testo; controlla il passaggio prima di confermare.",
        ja: "{target}は、この決定の影響を受ける部分としてマップに載ります。このリンクは文章\
             だけから見つかったものなので、確定する前に該当箇所を確認してください。",
        zh: "{target} 将在地图中显示为受该决策影响的部分。这条关联仅来自文字,确认前请检查相关片段。",
        ko: "{target}이(가) 이 결정의 영향을 받는 부분으로 맵에 표시돼요. 이 연결은 \
             텍스트에서만 나온 것이니, 확인하기 전에 해당 구절을 살펴보세요.",
        ru: "{target} будет значиться на карте как часть, затронутая этим решением. Связь \
             найдена только по тексту; проверьте фрагмент перед подтверждением." }
}

/// What confirming a link found only in the text of a rule or decision
/// changes; `uses` says whether the target is a technology, `rule` whether
/// the source is a rule.
pub fn link_mention_effect(target: &str, uses: bool, rule: bool) -> String {
    match (uses, rule) {
        (true, true) => link_mention_uses_rule_effect(target),
        (true, false) => link_mention_uses_decision_effect(target),
        (false, true) => link_mention_affects_rule_effect(target),
        (false, false) => link_mention_affects_decision_effect(target),
    }
}

plurals! {
    /// Sentence: decisions touched files no component covers.
    item_uncovered(count: usize, pattern: &str) {
        en: ["One decision touched files in {BOLD}{pattern}{BOLD} that no component on the \
             Map covers.",
            "Several decisions touched files in {BOLD}{pattern}{BOLD} that no component on \
             the Map covers."],
        pt: ["Uma decisão mexeu em arquivos de {BOLD}{pattern}{BOLD} que nenhum componente do \
             Mapa cobre.",
            "Várias decisões mexeram em arquivos de {BOLD}{pattern}{BOLD} que nenhum \
             componente do Mapa cobre."],
        es: ["Una decisión tocó archivos de {BOLD}{pattern}{BOLD} que ningún componente del \
             Mapa cubre.",
            "Varias decisiones tocaron archivos de {BOLD}{pattern}{BOLD} que ningún \
             componente del Mapa cubre."],
        fr: ["Une décision a touché des fichiers de {BOLD}{pattern}{BOLD} qu’aucun composant \
             de la Carte ne couvre.",
            "Plusieurs décisions ont touché des fichiers de {BOLD}{pattern}{BOLD} qu’aucun \
             composant de la Carte ne couvre."],
        de: ["Eine Entscheidung hat Dateien in {BOLD}{pattern}{BOLD} berührt, die keine \
             Komponente der Karte abdeckt.",
            "Mehrere Entscheidungen haben Dateien in {BOLD}{pattern}{BOLD} berührt, die \
             keine Komponente der Karte abdeckt."],
        it: ["Una decisione ha toccato file di {BOLD}{pattern}{BOLD} che nessun componente \
             della Mappa copre.",
            "Diverse decisioni hanno toccato file di {BOLD}{pattern}{BOLD} che nessun \
             componente della Mappa copre."],
        ja: "決定が {BOLD}{pattern}{BOLD} のファイルに触れましたが、 \
             マップのどのコンポーネントにも含まれていません。",
        zh: "有决策涉及了 {BOLD}{pattern}{BOLD} 中的文件， 但地图中没有任何组件覆盖它们。",
        ko: "결정이 {BOLD}{pattern}{BOLD}의 파일을 건드렸지만, 맵의 어떤 컴포넌트도 이를 \
             포함하지 않아요.",
        ru: ["Решение затронуло файлы в {BOLD}{pattern}{BOLD}, которые не охватывает ни один \
             компонент карты.",
            "Решения затронули файлы в {BOLD}{pattern}{BOLD}, которые не охватывает ни один \
             компонент карты.",
            "Решения затронули файлы в {BOLD}{pattern}{BOLD}, которые не охватывает ни один \
             компонент карты."]
    }
    /// What creating a component for uncovered files changes.
    item_uncovered_effect(count: usize, name: &str) {
        en: ["the component {name} enters the Map and {count} decision gets linked to it.",
            "the component {name} enters the Map and {count} decisions get linked to it."],
        pt: ["o componente {name} entra no Mapa e {count} decisão fica ligada a ele.",
            "o componente {name} entra no Mapa e {count} decisões ficam ligadas a ele."],
        es: ["el componente {name} entra en el Mapa y {count} decisión queda vinculada a él.",
            "el componente {name} entra en el Mapa y {count} decisiones quedan vinculadas \
             a él."],
        fr: ["le composant {name} entre dans la Carte et {count} décision lui est liée.",
            "le composant {name} entre dans la Carte et {count} décisions lui sont liées."],
        de: ["die Komponente {name} kommt in die Karte, und {count} Entscheidung wird mit ihr \
             verknüpft.",
            "die Komponente {name} kommt in die Karte, und {count} Entscheidungen werden mit \
             ihr verknüpft."],
        it: ["il componente {name} entra nella Mappa e {count} decisione viene collegata a \
             esso.",
            "il componente {name} entra nella Mappa e {count} decisioni vengono collegate a \
             esso."],
        ja: "コンポーネント{name}がマップに追加され、決定{count}件がリンクされます。",
        zh: "组件 {name} 将加入地图,{count} 项决策会与它关联。",
        ko: "컴포넌트({name})가 맵에 들어가고 결정 {count}개가 연결돼요.",
        ru: ["компонент {name} появится на карте, и {count} решение будет с ним связано.",
            "компонент {name} появится на карте, и {count} решения будут с ним связаны.",
            "компонент {name} появится на карте, и {count} решений будут с ним связаны."]
    }
    /// Sentence: a dependency added by decisions is not a map technology yet.
    item_technology(count: usize, name: &str) {
        en: ["The dependency {name} was added in {count} decision, but it isn’t a technology \
             on the Map yet.",
            "The dependency {name} was added in {count} decisions, but it isn’t a technology \
             on the Map yet."],
        pt: ["A dependência {name} foi adicionada em {count} decisão, mas ainda não é uma \
             tecnologia do Mapa.",
            "A dependência {name} foi adicionada em {count} decisões, mas ainda não é uma \
             tecnologia do Mapa."],
        es: ["La dependencia {name} se añadió en {count} decisión, pero aún no es una \
             tecnología del Mapa.",
            "La dependencia {name} se añadió en {count} decisiones, pero aún no es una \
             tecnología del Mapa."],
        fr: ["La dépendance {name} a été ajoutée dans {count} décision, mais ce n’est pas \
             encore une technologie de la Carte.",
            "La dépendance {name} a été ajoutée dans {count} décisions, mais ce n’est pas \
             encore une technologie de la Carte."],
        de: ["Die Abhängigkeit {name} wurde in {count} Entscheidung hinzugefügt, ist aber \
             noch keine Technologie der Karte.",
            "Die Abhängigkeit {name} wurde in {count} Entscheidungen hinzugefügt, ist aber \
             noch keine Technologie der Karte."],
        it: ["La dipendenza {name} è stata aggiunta in {count} decisione, ma non è ancora una \
             tecnologia della Mappa.",
            "La dipendenza {name} è stata aggiunta in {count} decisioni, ma non è ancora una \
             tecnologia della Mappa."],
        ja: "依存関係{name}が決定{count}件で追加されましたが、まだマップのテクノロジーでは\
             ありません。",
        zh: "依赖 {name} 已在 {count} 项决策中添加,但它还不是地图中的技术。",
        ko: "의존성 {name}: 결정 {count}개에서 추가됐지만 아직 맵의 기술이 아니에요.",
        ru: ["Зависимость {name} добавлена в {count} решении, но пока не является технологией \
             карты.",
            "Зависимость {name} добавлена в {count} решениях, но пока не является технологией \
             карты.",
            "Зависимость {name} добавлена в {count} решениях, но пока не является технологией \
             карты."]
    }
}

strings! {
    /// What confirming a dependency between decisions changes.
    relation_depends_effect {
        en: "the relation is recorded in the Relations section of both decisions, and whoever \
             reads the first sees which other one it depends on.",
        pt: "a relação fica registrada na seção Relações das duas decisões, e quem ler a \
             primeira vê de qual outra ela depende.",
        es: "la relación queda registrada en la sección Relaciones de ambas decisiones, y \
             quien lea la primera ve de cuál otra depende.",
        fr: "la relation est enregistrée dans la section Relations des deux décisions, et \
             quiconque lit la première voit de quelle autre elle dépend.",
        de: "die Beziehung wird im Abschnitt Beziehungen beider Entscheidungen erfasst, und \
             wer die erste liest, sieht, von welcher anderen sie abhängt.",
        it: "la relazione viene registrata nella sezione Relazioni di entrambe le decisioni, \
             e chi legge la prima vede da quale altra dipende.",
        ja: "関係は2つの決定の「関係」セクションに記録され、最初の決定を読む人は、どの決定に\
             依存しているかが分かります。",
        zh: "该关系会记录在两项决策的“关系”部分,阅读第一项决策的人可以看到它依赖于哪一项。",
        ko: "관계는 두 결정의 '관계' 섹션에 기록되고, 첫 번째 결정을 읽는 사람은 어떤 결정에 \
             의존하는지 볼 수 있어요.",
        ru: "связь записывается в разделе «Связи» обеих решений, и тот, кто читает первое, \
             видит, от какого другого оно зависит." }
    /// What confirming a conflict between decisions changes.
    relation_conflicts_effect {
        en: "the contradiction is recorded on both decisions; resolve it by superseding one \
             of them.",
        pt: "a contradição fica registrada nas duas decisões; resolva substituindo uma delas.",
        es: "la contradicción queda registrada en las dos decisiones; resuélvela sustituyendo \
             una de ellas.",
        fr: "la contradiction est enregistrée sur les deux décisions ; résolvez-la en \
             remplaçant l’une d’elles.",
        de: "der Widerspruch wird bei beiden Entscheidungen erfasst; löse ihn, indem du eine \
             davon ersetzt.",
        it: "la contraddizione viene registrata su entrambe le decisioni; risolvila \
             sostituendone una.",
        ja: "矛盾は両方の決定に記録されます。どちらかを置き換えて解消してください。",
        zh: "矛盾会记录在两项决策上;请通过取代其中一项来解决。",
        ko: "모순은 두 결정에 모두 기록돼요. 둘 중 하나를 대체해서 해결하세요.",
        ru: "противоречие записывается в обоих решениях; устраните его, заменив одно из них." }
    /// What confirming a replacement between decisions changes.
    relation_supersedes_effect {
        en: "the replacement is recorded in the Relations section of both decisions.",
        pt: "a substituição fica registrada na seção Relações das duas decisões.",
        es: "la sustitución queda registrada en la sección Relaciones de ambas decisiones.",
        fr: "le remplacement est enregistré dans la section Relations des deux décisions.",
        de: "die Ersetzung wird im Abschnitt Beziehungen beider Entscheidungen erfasst.",
        it: "la sostituzione viene registrata nella sezione Relazioni di entrambe le decisioni.",
        ja: "置き換えは2つの決定の「関係」セクションに記録されます。",
        zh: "取代关系会记录在两项决策的“关系”部分。",
        ko: "대체 관계는 두 결정의 '관계' 섹션에 기록돼요.",
        ru: "замена записывается в разделе «Связи» обеих решений." }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_alternate_plain_and_strong() {
        let text = format!("a {BOLD}b c{BOLD} d");
        assert_eq!(
            spans(&text),
            [
                ("a ".to_owned(), false),
                ("b c".to_owned(), true),
                (" d".to_owned(), false)
            ]
        );
        assert_eq!(spans("plain"), [("plain".to_owned(), false)]);
    }

    #[test]
    fn sentences_carry_their_names() {
        let text = relation_conflicts("A", "B");
        let strong: Vec<String> = spans(&text)
            .into_iter()
            .filter(|(_, strong)| *strong)
            .map(|(text, _)| text)
            .collect();
        assert_eq!(strong, ["“A”", "“B”"]);
    }
}
