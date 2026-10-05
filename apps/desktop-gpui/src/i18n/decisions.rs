//! Copy for the decisions area. See [`crate::i18n`] for how entries are declared.

use super::{formats, strings};

strings! {
    /// Placeholder and label of the index search.
    search_context { en: "Search decisions", pt: "Buscar nas decisões",
        es: "Buscar en decisiones", fr: "Rechercher dans les décisions",
        de: "Entscheidungen durchsuchen", it: "Cerca nelle decisioni",
        ja: "決定を検索", zh: "搜索决策", ko: "결정 검색", ru: "Поиск по решениям" }
    /// Question shown for a related decision that cannot be read.
    unavailable_decision { en: "Decision unavailable", pt: "Decisão indisponível",
        es: "Decisión no disponible", fr: "Décision indisponible",
        de: "Entscheidung nicht verfügbar", it: "Decisione non disponibile",
        ja: "決定は利用できません", zh: "决策不可用", ko: "결정을 사용할 수 없어요",
        ru: "Решение недоступно" }
    /// Error: the index could not be loaded.
    load_failed { en: "Couldn't load the decisions.",
        pt: "Não foi possível carregar as decisões.",
        es: "No se pudieron cargar las decisiones.",
        fr: "Impossible de charger les décisions.",
        de: "Die Entscheidungen konnten nicht geladen werden.",
        it: "Impossibile caricare le decisioni.",
        ja: "決定を読み込めませんでした。", zh: "无法加载决策。",
        ko: "결정을 불러오지 못했어요.", ru: "Не удалось загрузить решения." }
    /// Toast: a revision was saved.
    revision_saved { en: "New version saved. History preserved.",
        pt: "Nova versão salva. Histórico preservado.",
        es: "Nueva versión guardada. Historial conservado.",
        fr: "Nouvelle version enregistrée. Historique conservé.",
        de: "Neue Version gespeichert. Verlauf bleibt erhalten.",
        it: "Nuova versione salvata. Cronologia conservata.",
        ja: "新しいバージョンを保存しました。履歴は保持されます。",
        zh: "已保存新版本。历史已保留。",
        ko: "새 버전을 저장했어요. 기록은 그대로 보존돼요.",
        ru: "Новая версия сохранена. История сохранена." }
    /// Error: the export file could not be written.
    save_file_failed { en: "Couldn't save the file. Choose another destination and try again.",
        pt: "Não foi possível salvar o arquivo. Escolha outro destino e tente novamente.",
        es: "No se pudo guardar el archivo. Elige otro destino e inténtalo de nuevo.",
        fr: "Impossible d’enregistrer le fichier. Choisissez une autre destination et réessayez.",
        de: "Die Datei konnte nicht gespeichert werden. Wähle ein anderes Ziel und versuche es erneut.",
        it: "Impossibile salvare il file. Scegli un’altra destinazione e riprova.",
        ja: "ファイルを保存できませんでした。別の保存先を選んで、もう一度お試しください。",
        zh: "无法保存文件。请选择其他位置后重试。",
        ko: "파일을 저장하지 못했어요. 다른 위치를 선택하고 다시 시도해 주세요.",
        ru: "Не удалось сохранить файл. Выберите другое место и повторите попытку." }
    /// Toast: the source excerpt was copied.
    excerpt_copied { en: "Excerpt copied.", pt: "Trecho copiado.", es: "Fragmento copiado.",
        fr: "Extrait copié.", de: "Ausschnitt kopiert.", it: "Estratto copiato.",
        ja: "抜粋をコピーしました。", zh: "已复制片段。", ko: "발췌문을 복사했어요.",
        ru: "Фрагмент скопирован." }
    /// Error: a decision changed state while relating.
    relation_conflict { en: "One of the decisions changed state. The list was updated.",
        pt: "Uma das decisões mudou de estado. A lista foi atualizada.",
        es: "Una de las decisiones cambió de estado. Se actualizó la lista.",
        fr: "Une des décisions a changé d’état. La liste a été mise à jour.",
        de: "Eine der Entscheidungen hat ihren Status geändert. Die Liste wurde aktualisiert.",
        it: "Una delle decisioni ha cambiato stato. L’elenco è stato aggiornato.",
        ja: "どちらかの決定の状態が変わりました。一覧を更新しました。",
        zh: "其中一项决策的状态已变化。列表已更新。",
        ko: "결정 중 하나의 상태가 바뀌었어요. 목록을 새로 고쳤어요.",
        ru: "Состояние одного из решений изменилось. Список обновлён." }
    /// Error: the relation could not be recorded.
    relation_failed { en: "Couldn't record the relation.",
        pt: "Não foi possível registrar a relação.",
        es: "No se pudo registrar la relación.", fr: "Impossible d’enregistrer la relation.",
        de: "Die Beziehung konnte nicht erfasst werden.",
        it: "Impossibile registrare la relazione.", ja: "関係を記録できませんでした。",
        zh: "无法记录关系。", ko: "관계를 기록하지 못했어요.",
        ru: "Не удалось записать связь." }
    /// Error: the export preview could not be built.
    export_prepare_failed { en: "Couldn't prepare the export.",
        pt: "Não foi possível preparar a exportação.",
        es: "No se pudo preparar la exportación.", fr: "Impossible de préparer l’export.",
        de: "Der Export konnte nicht vorbereitet werden.",
        it: "Impossibile preparare l’esportazione.",
        ja: "エクスポートを準備できませんでした。", zh: "无法准备导出。",
        ko: "내보내기를 준비하지 못했어요.", ru: "Не удалось подготовить экспорт." }
    /// Title of the export file dialog.
    export_dialog_title { en: "Export decision", pt: "Exportar decisão",
        es: "Exportar decisión", fr: "Exporter la décision",
        de: "Entscheidung exportieren", it: "Esporta decisione", ja: "決定をエクスポート",
        zh: "导出决策", ko: "결정 내보내기", ru: "Экспорт решения" }
    /// Name of the file type in the export dialog.
    export_dialog_filter { en: "Document", pt: "Documento", es: "Documento", fr: "Document",
        de: "Dokument", it: "Documento", ja: "ドキュメント", zh: "文档", ko: "문서",
        ru: "Документ" }
    /// Start of the suggested export file name (followed by the decision id).
    export_file_prefix { en: "decision", pt: "decisao", es: "decision", fr: "decision",
        de: "entscheidung", it: "decisione", ja: "decision", zh: "decision", ko: "decision",
        ru: "decision" }
    /// Error: revising failed, most likely a version conflict.
    revise_failed { en: "Couldn't save. The decision may have received another version; cancel and refresh before trying again.",
        pt: "Não foi possível salvar. A decisão pode ter recebido outra versão; cancele e atualize antes de tentar novamente.",
        es: "No se pudo guardar. La decisión puede haber recibido otra versión; cancela y actualiza antes de intentarlo de nuevo.",
        fr: "Enregistrement impossible. La décision a peut-être reçu une autre version ; annulez et actualisez avant de réessayer.",
        de: "Speichern nicht möglich. Die Entscheidung hat womöglich eine andere Version erhalten; brich ab und aktualisiere, bevor du es erneut versuchst.",
        it: "Impossibile salvare. La decisione potrebbe aver ricevuto un’altra versione; annulla e aggiorna prima di riprovare.",
        ja: "保存できませんでした。決定に別のバージョンが追加された可能性があります。キャンセルして更新してから、もう一度お試しください。",
        zh: "无法保存。该决策可能已有新版本；请先取消并刷新，再重试。",
        ko: "저장하지 못했어요. 결정에 다른 버전이 생겼을 수 있어요. 취소하고 새로 고친 뒤 다시 시도해 주세요.",
        ru: "Не удалось сохранить. Возможно, у решения появилась другая версия; отмените и обновите, прежде чем повторять." }
    /// Index filter while searching.
    filter_all_searching { en: "All states · search", pt: "Todos os estados · busca",
        es: "Todos los estados · búsqueda", fr: "Tous les états · recherche",
        de: "Alle Status · Suche", it: "Tutti gli stati · ricerca",
        ja: "すべての状態 · 検索", zh: "所有状态 · 搜索", ko: "모든 상태 · 검색",
        ru: "Все состояния · поиск" }
    /// Index filter while a part is chosen.
    filter_part { en: "In effect · part", pt: "Em vigor · parte", es: "Vigentes · parte",
        fr: "En vigueur · partie", de: "In Kraft · Teil", it: "In vigore · parte",
        ja: "有効 · パート", zh: "生效中 · 部分", ko: "유효 · 파트", ru: "Действует · часть" }
    /// Index filter showing every state.
    filter_all { en: "All states", pt: "Todos os estados", es: "Todos los estados",
        fr: "Tous les états", de: "Alle Status", it: "Tutti gli stati", ja: "すべての状態",
        zh: "所有状态", ko: "모든 상태", ru: "Все состояния" }
    /// Index filter showing confirmed decisions only.
    filter_confirmed { en: "Confirmed", pt: "Confirmadas", es: "Confirmadas",
        fr: "Confirmées", de: "Bestätigt", it: "Confermate", ja: "確定済み", zh: "已确认",
        ko: "확인됨", ru: "Подтверждённые" }
    /// Title of the index panel.
    index_title { en: "Index", pt: "Índice", es: "Índice", fr: "Index", de: "Index",
        it: "Indice", ja: "インデックス", zh: "索引", ko: "색인", ru: "Индекс" }
    /// Note replacing the search while an editor or preview is open.
    index_locked { en: "Finish or cancel to browse the index.",
        pt: "Conclua ou cancele para navegar no índice.",
        es: "Termina o cancela para navegar por el índice.",
        fr: "Terminez ou annulez pour parcourir l’index.",
        de: "Schließe ab oder brich ab, um im Index zu navigieren.",
        it: "Termina o annulla per navigare nell’indice.",
        ja: "完了またはキャンセルすると、インデックスを移動できます。",
        zh: "完成或取消后才能浏览索引。",
        ko: "완료하거나 취소하면 색인을 이동할 수 있어요.",
        ru: "Завершите или отмените, чтобы перемещаться по индексу." }
    /// Footer of the index while loading.
    loading { en: "Loading…", pt: "Carregando…", es: "Cargando…", fr: "Chargement…",
        de: "Wird geladen…", it: "Caricamento…", ja: "読み込み中…", zh: "加载中…",
        ko: "불러오는 중…", ru: "Загрузка…" }
    /// Label of the menu trigger with no part chosen, and its first option.
    all_parts { en: "All parts", pt: "Todas as partes", es: "Todas las partes",
        fr: "Toutes les parties", de: "Alle Teile", it: "Tutte le parti",
        ja: "すべてのパート", zh: "所有部分", ko: "모든 파트", ru: "Все части" }
    /// Button and tooltip that clear the part filter.
    show_all_parts { en: "Show all parts", pt: "Mostrar todas as partes",
        es: "Mostrar todas las partes", fr: "Afficher toutes les parties",
        de: "Alle Teile anzeigen", it: "Mostra tutte le parti",
        ja: "すべてのパートを表示", zh: "显示所有部分", ko: "모든 파트 보기",
        ru: "Показать все части" }
    /// Accessible name of the menu of parts.
    parts_menu_label { en: "Decisions by part", pt: "Decisões por parte",
        es: "Decisiones por parte", fr: "Décisions par partie",
        de: "Entscheidungen nach Teil", it: "Decisioni per parte", ja: "パート別の決定",
        zh: "按部分查看决策", ko: "파트별 결정", ru: "Решения по частям" }
    /// Hint of the menu of parts when the Map has none.
    no_parts_hint { en: "The Map has no parts yet. Each top-level component of the Map becomes a part here.",
        pt: "O Mapa ainda não tem partes. Cada componente de topo do Mapa vira uma parte aqui.",
        es: "El Mapa aún no tiene partes. Cada componente de primer nivel del Mapa se convierte en una parte aquí.",
        fr: "La Carte n’a pas encore de parties. Chaque composant de premier niveau de la Carte devient une partie ici.",
        de: "Die Karte hat noch keine Teile. Jede Komponente der obersten Ebene der Karte wird hier zu einem Teil.",
        it: "La Mappa non ha ancora parti. Ogni componente di primo livello della Mappa diventa una parte qui.",
        ja: "マップにはまだパートがありません。マップの最上位のコンポーネントが、ここでパートになります。",
        zh: "地图中还没有部分。地图的每个顶层组件都会在这里成为一个部分。",
        ko: "맵에 아직 파트가 없어요. 맵의 최상위 컴포넌트마다 여기에서 파트가 돼요.",
        ru: "На карте пока нет частей. Каждый компонент верхнего уровня карты становится здесь частью." }
    /// Button that loads the next page of the index.
    load_more { en: "Load more", pt: "Carregar mais", es: "Cargar más", fr: "Charger plus",
        de: "Mehr laden", it: "Carica altri", ja: "さらに読み込む", zh: "加载更多",
        ko: "더 불러오기", ru: "Загрузить ещё" }
    /// Empty index: no search, no part.
    empty_index { en: "Confirmed decisions will appear here.",
        pt: "As decisões confirmadas aparecerão aqui.",
        es: "Las decisiones confirmadas aparecerán aquí.",
        fr: "Les décisions confirmées apparaîtront ici.",
        de: "Bestätigte Entscheidungen erscheinen hier.",
        it: "Le decisioni confermate appariranno qui.",
        ja: "確定した決定はここに表示されます。", zh: "已确认的决策会显示在这里。",
        ko: "확인한 결정이 여기에 표시돼요.", ru: "Подтверждённые решения появятся здесь." }
    /// Empty index: a search with no result.
    empty_search { en: "No results. Try another word.",
        pt: "Nenhum resultado. Tente outra palavra.",
        es: "Sin resultados. Prueba otra palabra.",
        fr: "Aucun résultat. Essayez un autre mot.",
        de: "Keine Treffer. Probiere ein anderes Wort.",
        it: "Nessun risultato. Prova un’altra parola.",
        ja: "結果がありません。別の言葉をお試しください。",
        zh: "没有结果。试试其他词。", ko: "결과가 없어요. 다른 단어를 입력해 보세요.",
        ru: "Ничего не найдено. Попробуйте другое слово." }
    /// State pill of a superseded decision.
    superseded { en: "Superseded", pt: "Substituída", es: "Sustituida", fr: "Remplacée",
        de: "Ersetzt", it: "Sostituita", ja: "置き換え済み", zh: "已取代", ko: "대체됨",
        ru: "Заменено" }
    /// State pill of a confirmed decision.
    confirmed_badge { en: "Confirmed", pt: "Confirmada", es: "Confirmada", fr: "Confirmée",
        de: "Bestätigt", it: "Confermata", ja: "確定済み", zh: "已确认", ko: "확인됨",
        ru: "Подтверждено" }
    /// Kicker of the empty panel of a part.
    part_kicker { en: "Part", pt: "Parte", es: "Parte", fr: "Partie", de: "Teil",
        it: "Parte", ja: "パート", zh: "部分", ko: "파트", ru: "Часть" }
    /// Kicker of the empty panel with no decisions at all.
    empty_kicker { en: "Decisions", pt: "Decisões", es: "Decisiones", fr: "Décisions",
        de: "Entscheidungen", it: "Decisioni", ja: "決定", zh: "决策", ko: "결정",
        ru: "Решения" }
    /// Title of the empty panel with no decisions at all.
    empty_title { en: "Decisions that last", pt: "Decisões que permanecem",
        es: "Decisiones que permanecen", fr: "Des décisions qui durent",
        de: "Entscheidungen, die bleiben", it: "Decisioni che restano",
        ja: "残り続ける決定", zh: "长期保留的决策", ko: "계속 남는 결정",
        ru: "Решения, которые остаются" }
    /// Body of the empty panel with no decisions at all.
    empty_body { en: "Confirm a choice in Review to keep the document, its evidence and its history here.",
        pt: "Confirme uma escolha na Revisão para preservar o documento, suas evidências e seu histórico aqui.",
        es: "Confirma una elección en Revisión para conservar aquí el documento, sus evidencias y su historial.",
        fr: "Confirmez un choix dans Revue pour conserver ici le document, ses preuves et son historique.",
        de: "Bestätige eine Wahl in der Prüfung, um das Dokument, seine Belege und seinen Verlauf hier zu bewahren.",
        it: "Conferma una scelta in Revisione per conservare qui il documento, le sue evidenze e la sua cronologia.",
        ja: "レビューで選択を確定すると、ドキュメントとその根拠、履歴がここに残ります。",
        zh: "在审阅中确认一项选择，即可在这里保留文档、证据和历史。",
        ko: "검토에서 선택을 확인하면 문서와 근거, 기록이 여기에 보존돼요.",
        ru: "Подтвердите выбор в проверке, и документ, его доказательства и история сохранятся здесь." }
    /// Kicker of the empty panel of a search.
    search_kicker { en: "Search", pt: "Busca", es: "Búsqueda", fr: "Recherche",
        de: "Suche", it: "Ricerca", ja: "検索", zh: "搜索", ko: "검색", ru: "Поиск" }
    /// Title of the empty panel of a search.
    search_empty_title { en: "No decision found", pt: "Nenhuma decisão encontrada",
        es: "Ninguna decisión encontrada", fr: "Aucune décision trouvée",
        de: "Keine Entscheidung gefunden", it: "Nessuna decisione trovata",
        ja: "決定が見つかりません", zh: "未找到决策", ko: "결정을 찾을 수 없어요",
        ru: "Решения не найдены" }
    /// Body of the empty panel of a search.
    search_empty_body { en: "Search covers the question, choice and rationale in this project. Try another word.",
        pt: "A busca cobre pergunta, escolha e justificativa deste projeto. Tente outra palavra.",
        es: "La búsqueda abarca la pregunta, la elección y la justificación de este proyecto. Prueba otra palabra.",
        fr: "La recherche porte sur la question, le choix et la justification de ce projet. Essayez un autre mot.",
        de: "Die Suche umfasst Frage, Wahl und Begründung dieses Projekts. Probiere ein anderes Wort.",
        it: "La ricerca copre domanda, scelta e motivazione di questo progetto. Prova un’altra parola.",
        ja: "検索対象は、このプロジェクトの質問・選択・判断理由です。別の言葉をお試しください。",
        zh: "搜索范围包括此项目的问题、选择和理由。试试其他词。",
        ko: "이 프로젝트의 질문, 선택, 판단 이유를 검색해요. 다른 단어를 입력해 보세요.",
        ru: "Поиск охватывает вопрос, выбор и обоснование в этом проекте. Попробуйте другое слово." }
    /// Button back to the document from the history.
    back_to_document { en: "Back to document", pt: "Voltar ao documento",
        es: "Volver al documento", fr: "Retour au document", de: "Zurück zum Dokument",
        it: "Torna al documento", ja: "ドキュメントに戻る", zh: "返回文档",
        ko: "문서로 돌아가기", ru: "Назад к документу" }
    /// Title of the version history.
    history_title { en: "Version history", pt: "Histórico de versões",
        es: "Historial de versiones", fr: "Historique des versions",
        de: "Versionsverlauf", it: "Cronologia delle versioni", ja: "バージョン履歴",
        zh: "版本历史", ko: "버전 기록", ru: "История версий" }
    /// Explanation under the title of the version history.
    history_hint { en: "Each version keeps the full document. Open a version to read its content.",
        pt: "Cada versão conserva o documento completo. Abra uma versão para ler seu conteúdo.",
        es: "Cada versión conserva el documento completo. Abre una versión para leer su contenido.",
        fr: "Chaque version conserve le document complet. Ouvrez une version pour lire son contenu.",
        de: "Jede Version enthält das vollständige Dokument. Öffne eine Version, um ihren Inhalt zu lesen.",
        it: "Ogni versione conserva il documento completo. Apri una versione per leggerne il contenuto.",
        ja: "各バージョンにはドキュメント全体が保存されます。バージョンを開くと内容を読めます。",
        zh: "每个版本都保留完整文档。打开某个版本即可阅读其内容。",
        ko: "각 버전은 문서 전체를 보존해요. 버전을 열면 내용을 읽을 수 있어요.",
        ru: "Каждая версия хранит документ целиком. Откройте версию, чтобы прочитать её содержимое." }
    /// Button that exports the decision.
    export_open { en: "Export…", pt: "Exportar…", es: "Exportar…", fr: "Exporter…",
        de: "Exportieren…", it: "Esporta…", ja: "エクスポート…", zh: "导出…", ko: "내보내기…",
        ru: "Экспорт…" }
    /// Button that opens the revision form.
    revise { en: "Revise", pt: "Revisar", es: "Revisar", fr: "Réviser", de: "Überarbeiten",
        it: "Rivedi", ja: "改訂", zh: "修订", ko: "수정", ru: "Пересмотреть" }
    /// Button back to the current version from an older one.
    back_to_current { en: "Back to current version", pt: "Voltar à versão atual",
        es: "Volver a la versión actual", fr: "Retour à la version actuelle",
        de: "Zurück zur aktuellen Version", it: "Torna alla versione attuale",
        ja: "現在のバージョンに戻る", zh: "返回当前版本", ko: "현재 버전으로 돌아가기",
        ru: "Назад к текущей версии" }
    /// Banner shown while an older version is open.
    historical_note { en: "Historical version, read-only. Revise and export use the current version.",
        pt: "Versão histórica, somente leitura. Revisar e exportar usam a versão atual.",
        es: "Versión histórica, solo lectura. Revisar y exportar usan la versión actual.",
        fr: "Version historique, en lecture seule. Réviser et exporter utilisent la version actuelle.",
        de: "Frühere Version, schreibgeschützt. Überarbeiten und Exportieren verwenden die aktuelle Version.",
        it: "Versione storica, sola lettura. Rivedi ed Esporta usano la versione attuale.",
        ja: "過去のバージョンで、読み取り専用です。改訂とエクスポートは現在のバージョンを使います。",
        zh: "历史版本，只读。修订和导出使用当前版本。",
        ko: "이전 버전이라 읽기만 할 수 있어요. 수정과 내보내기는 현재 버전을 사용해요.",
        ru: "Историческая версия, только чтение. Пересмотр и экспорт используют текущую версию." }
    /// Label of the confirmed choice in the document.
    confirmed_choice { en: "Confirmed choice", pt: "Escolha confirmada",
        es: "Elección confirmada", fr: "Choix confirmé", de: "Bestätigte Wahl",
        it: "Scelta confermata", ja: "確定した選択", zh: "已确认的选择",
        ko: "확인한 선택", ru: "Подтверждённый выбор" }
    /// Field: the question of the decision.
    label_question { en: "Question", pt: "Pergunta", es: "Pregunta", fr: "Question",
        de: "Frage", it: "Domanda", ja: "質問", zh: "问题", ko: "질문", ru: "Вопрос" }
    /// Field: the choice of the decision.
    label_choice { en: "Choice", pt: "Escolha", es: "Elección", fr: "Choix", de: "Wahl",
        it: "Scelta", ja: "選択", zh: "选择", ko: "선택", ru: "Выбор" }
    /// Field: why the choice was made.
    label_rationale { en: "Rationale", pt: "Justificativa", es: "Justificación",
        fr: "Justification", de: "Begründung", it: "Motivazione", ja: "判断理由",
        zh: "理由", ko: "판단 이유", ru: "Обоснование" }
    /// Field: the assumptions behind the decision.
    label_assumptions { en: "Assumptions", pt: "Premissas", es: "Supuestos",
        fr: "Hypothèses", de: "Annahmen", it: "Presupposti", ja: "前提", zh: "前提",
        ko: "전제", ru: "Допущения" }
    /// Field: the scope of the decision.
    label_scope { en: "Scope", pt: "Escopo", es: "Alcance", fr: "Portée",
        de: "Geltungsbereich", it: "Ambito", ja: "範囲", zh: "范围", ko: "범위",
        ru: "Область" }
    /// Field: the consequences of the decision.
    label_consequences { en: "Consequences", pt: "Consequências", es: "Consecuencias",
        fr: "Conséquences", de: "Folgen", it: "Conseguenze", ja: "影響", zh: "影响",
        ko: "영향", ru: "Последствия" }
    /// Field: when to reconsider the decision.
    label_reconsider_when { en: "Reconsider when", pt: "Reconsiderar quando",
        es: "Reconsiderar cuando", fr: "Réexaminer quand", de: "Erneut prüfen, wenn",
        it: "Riconsiderare quando", ja: "見直す条件", zh: "重新考虑的时机",
        ko: "다시 검토할 때", ru: "Пересмотреть, когда" }
    /// Inside a disclosure with no items.
    no_items { en: "No items recorded.", pt: "Nenhum item registrado.",
        es: "Ningún elemento registrado.", fr: "Aucun élément enregistré.",
        de: "Keine Einträge erfasst.", it: "Nessuna voce registrata.",
        ja: "登録された項目はありません。", zh: "没有已记录的条目。",
        ko: "기록된 항목이 없어요.", ru: "Записей нет." }
    /// Section header of the decision's context disclosures.
    context_title { en: "Decision context", pt: "Contexto da decisão",
        es: "Contexto de la decisión", fr: "Contexte de la décision",
        de: "Kontext der Entscheidung", it: "Contesto della decisione",
        ja: "決定のコンテキスト", zh: "决策上下文", ko: "결정 컨텍스트",
        ru: "Контекст решения" }
    /// Section label: where the decision came from.
    origin { en: "Origin", pt: "Origem", es: "Origen", fr: "Origine", de: "Herkunft",
        it: "Origine", ja: "由来", zh: "起源", ko: "기원", ru: "Происхождение" }
    /// Origin of a decision confirmed from a capture.
    origin_captured { en: "Confirmed in Review from a captured conversation.",
        pt: "Confirmada na Revisão a partir de uma conversa capturada.",
        es: "Confirmada en Revisión a partir de una conversación capturada.",
        fr: "Confirmée dans Revue à partir d’une conversation capturée.",
        de: "In der Prüfung aus einer erfassten Unterhaltung bestätigt.",
        it: "Confermata in Revisione a partire da una conversazione acquisita.",
        ja: "キャプチャした会話から、レビューで確定しました。",
        zh: "在审阅中根据捕获的对话确认。",
        ko: "캡처한 대화를 바탕으로 검토에서 확인했어요.",
        ru: "Подтверждено в проверке по захваченному разговору." }
    /// Origin of a decision whose capture was not recorded.
    origin_uncaptured { en: "Confirmed in Review; the source capture wasn’t recorded.",
        pt: "Confirmada na Revisão; a captura de origem não foi registrada.",
        es: "Confirmada en Revisión; no se registró la captura de origen.",
        fr: "Confirmée dans Revue ; la capture d’origine n’a pas été enregistrée.",
        de: "In der Prüfung bestätigt; die ursprüngliche Erfassung wurde nicht aufgezeichnet.",
        it: "Confermata in Revisione; l’acquisizione di origine non è stata registrata.",
        ja: "レビューで確定しました。元のキャプチャは記録されていません。",
        zh: "在审阅中确认；未记录来源捕获。",
        ko: "검토에서 확인했어요. 원본 캡처는 기록되지 않았어요.",
        ru: "Подтверждено в проверке; исходный захват не записан." }
    /// Section header of the relations of the decision.
    relations { en: "Relations", pt: "Relações", es: "Relaciones", fr: "Relations",
        de: "Beziehungen", it: "Relazioni", ja: "関係", zh: "关系", ko: "관계",
        ru: "Связи" }
    /// Button that starts relating the decision to another.
    relate { en: "Relate", pt: "Relacionar", es: "Relacionar", fr: "Relier",
        de: "Verknüpfen", it: "Collega", ja: "関係を追加", zh: "建立关系", ko: "관계 추가",
        ru: "Связать" }
    /// Empty relations of a decision in effect.
    no_relations_accepted { en: "No relations. Link this decision to those it supersedes, depends on or conflicts with.",
        pt: "Sem relações. Ligue esta decisão às que ela substitui, das quais depende ou com que conflita.",
        es: "Sin relaciones. Vincula esta decisión con las que sustituye, de las que depende o con las que entra en conflicto.",
        fr: "Aucune relation. Reliez cette décision à celles qu’elle remplace, dont elle dépend ou avec lesquelles elle est en conflit.",
        de: "Keine Beziehungen. Verknüpfe diese Entscheidung mit denen, die sie ersetzt, von denen sie abhängt oder zu denen sie im Widerspruch steht.",
        it: "Nessuna relazione. Collega questa decisione a quelle che sostituisce, da cui dipende o con cui è in conflitto.",
        ja: "関係はありません。この決定を、置き換えるもの、依存するもの、競合するものにリンクしましょう。",
        zh: "暂无关系。可将此决策关联到它所取代、依赖或与之冲突的决策。",
        ko: "관계가 없어요. 이 결정이 대체하거나 의존하거나 충돌하는 결정에 연결해 보세요.",
        ru: "Связей нет. Свяжите это решение с теми, которые оно заменяет, от которых зависит или с которыми конфликтует." }
    /// Empty relations of a superseded decision.
    no_relations { en: "No relations recorded.", pt: "Sem relações registradas.",
        es: "Sin relaciones registradas.", fr: "Aucune relation enregistrée.",
        de: "Keine Beziehungen erfasst.", it: "Nessuna relazione registrata.",
        ja: "登録された関係はありません。", zh: "没有已记录的关系。",
        ko: "기록된 관계가 없어요.", ru: "Связей не записано." }
    /// Hint of the "depends on" relation.
    hint_depends { en: "This decision holds only while the chosen one holds.",
        pt: "Esta decisão só vale enquanto a escolhida valer.",
        es: "Esta decisión solo vale mientras valga la elegida.",
        fr: "Cette décision ne vaut que tant que celle choisie vaut.",
        de: "Diese Entscheidung gilt nur, solange die gewählte gilt.",
        it: "Questa decisione vale solo finché vale quella scelta.",
        ja: "この決定は、選んだ決定が有効な間だけ有効です。",
        zh: "只有所选决策仍然有效时，这项决策才有效。",
        ko: "이 결정은 선택한 결정이 유효한 동안만 유효해요.",
        ru: "Это решение действует, пока действует выбранное." }
    /// Hint of the "conflicts with" relation.
    hint_conflicts { en: "The two can’t hold together; the relation shows on both.",
        pt: "As duas não podem valer juntas; a relação aparece nas duas.",
        es: "Las dos no pueden valer juntas; la relación aparece en ambas.",
        fr: "Les deux ne peuvent pas valoir ensemble ; la relation apparaît sur les deux.",
        de: "Beide können nicht gleichzeitig gelten; die Beziehung erscheint bei beiden.",
        it: "Le due non possono valere insieme; la relazione compare su entrambe.",
        ja: "両方を同時に有効にはできません。関係は両方に表示されます。",
        zh: "两者不能同时有效；该关系会显示在两项决策上。",
        ko: "둘은 함께 유효할 수 없어요. 관계는 양쪽에 표시돼요.",
        ru: "Обе не могут действовать одновременно; связь видна в обеих." }
    /// Hint of the "supersedes" relation.
    hint_supersedes { en: "The chosen one becomes Superseded, leaves the agent’s context and stays in history.",
        pt: "A escolhida passa a Substituída, sai do contexto do agente e fica no histórico.",
        es: "La elegida pasa a Sustituida, sale del contexto del agente y queda en el historial.",
        fr: "La décision choisie devient Remplacée, sort du contexte de l’agent et reste dans l’historique.",
        de: "Die gewählte wird Ersetzt, verlässt den Kontext des Agenten und bleibt im Verlauf.",
        it: "Quella scelta diventa Sostituita, esce dal contesto dell’agente e resta nella cronologia.",
        ja: "選んだ決定は「置き換え済み」になり、エージェントのコンテキストから外れて履歴に残ります。",
        zh: "所选决策会变为“已取代”，不再进入智能体的上下文，并保留在历史中。",
        ko: "선택한 결정은 '대체됨'이 되어 에이전트의 컨텍스트에서 빠지고 기록에 남아요.",
        ru: "Выбранное решение станет «Заменено», уйдёт из контекста агента и останется в истории." }
    /// Relation: this decision depends on the other.
    rel_depends_on { en: "Depends on", pt: "Depende de", es: "Depende de", fr: "Dépend de",
        de: "Hängt ab von", it: "Dipende da", ja: "依存先", zh: "依赖于", ko: "의존",
        ru: "Зависит от" }
    /// Relation: the two conflict.
    rel_conflicts { en: "Conflicts with", pt: "Conflita com", es: "Conflicto con",
        fr: "Conflit avec", de: "Widerspricht", it: "Conflitto con", ja: "競合先",
        zh: "冲突于", ko: "충돌", ru: "Конфликтует с" }
    /// Relation: this decision supersedes the other.
    rel_supersedes { en: "Supersedes", pt: "Substitui", es: "Sustituye a", fr: "Remplace",
        de: "Ersetzt", it: "Sostituisce", ja: "置き換える", zh: "取代", ko: "대체함",
        ru: "Заменяет" }
    /// Relation: the other decision supersedes this one.
    rel_superseded_by { en: "Superseded by", pt: "Substituída por", es: "Sustituida por",
        fr: "Remplacée par", de: "Ersetzt durch", it: "Sostituita da", ja: "置き換え元",
        zh: "被取代于", ko: "대체한 결정", ru: "Заменено на" }
    /// Relation: the other decision depends on this one.
    rel_basis_of { en: "Basis of", pt: "É base de", es: "Base de", fr: "Base de",
        de: "Grundlage von", it: "Base di", ja: "依存される", zh: "被依赖于",
        ko: "기반이 됨", ru: "Основа для" }
    /// Cancel button.
    cancel { en: "Cancel", pt: "Cancelar", es: "Cancelar", fr: "Annuler",
        de: "Abbrechen", it: "Annulla", ja: "キャンセル", zh: "取消", ko: "취소",
        ru: "Отмена" }
    /// Picker of the decision to relate: nothing to pick.
    no_candidates { en: "No other decision in effect loaded to relate.",
        pt: "Nenhuma outra decisão em vigor carregada para relacionar.",
        es: "No hay otra decisión vigente cargada para relacionar.",
        fr: "Aucune autre décision en vigueur chargée à relier.",
        de: "Keine weitere Entscheidung in Kraft geladen, die verknüpft werden könnte.",
        it: "Nessun’altra decisione in vigore caricata da collegare.",
        ja: "関連付けられる有効な決定が、ほかに読み込まれていません。",
        zh: "没有已加载的其他生效决策可供关联。",
        ko: "연결할 수 있는 다른 유효한 결정이 불러와져 있지 않아요.",
        ru: "Нет других загруженных действующих решений для связи." }
    /// Accessible name of the relation type group.
    relation_kind_aria { en: "Relation type", pt: "Tipo de relação",
        es: "Tipo de relación", fr: "Type de relation", de: "Art der Beziehung",
        it: "Tipo di relazione", ja: "関係の種類", zh: "关系类型", ko: "관계 유형",
        ru: "Тип связи" }
    /// Section header of the evidence of the decision.
    evidences { en: "Evidence", pt: "Evidências", es: "Evidencias", fr: "Preuves",
        de: "Belege", it: "Evidenze", ja: "根拠", zh: "证据", ko: "근거",
        ru: "Доказательства" }
    /// No source linked to the decision.
    no_sources { en: "No linked source.", pt: "Nenhuma fonte vinculada.",
        es: "Ninguna fuente vinculada.", fr: "Aucune source liée.",
        de: "Keine Quelle verknüpft.", it: "Nessuna fonte collegata.",
        ja: "リンクされたソースはありません。", zh: "没有关联的来源。",
        ko: "연결된 출처가 없어요.", ru: "Связанных источников нет." }
    /// Button that copies the source excerpt.
    copy_excerpt { en: "Copy excerpt", pt: "Copiar trecho", es: "Copiar fragmento",
        fr: "Copier l’extrait", de: "Ausschnitt kopieren", it: "Copia estratto",
        ja: "抜粋をコピー", zh: "复制片段", ko: "발췌문 복사", ru: "Копировать фрагмент" }
    /// Button that shrinks the expanded source.
    collapse { en: "Collapse", pt: "Recolher", es: "Contraer", fr: "Réduire",
        de: "Einklappen", it: "Comprimi", ja: "折りたたむ", zh: "收起", ko: "접기",
        ru: "Свернуть" }
    /// Button that expands the source reading area.
    expand_reading { en: "Expand reading", pt: "Ampliar leitura", es: "Ampliar lectura",
        fr: "Agrandir la lecture", de: "Lesebereich vergrößern", it: "Amplia lettura",
        ja: "表示を広げる", zh: "放大阅读", ko: "넓게 보기", ru: "Расширить чтение" }
    /// A source whose content cannot be read.
    source_unavailable { en: "Source unavailable. The provenance link was preserved.",
        pt: "Fonte indisponível. O vínculo de proveniência foi preservado.",
        es: "Fuente no disponible. Se conservó el vínculo de procedencia.",
        fr: "Source indisponible. Le lien de provenance a été conservé.",
        de: "Quelle nicht verfügbar. Die Verknüpfung zur Herkunft bleibt erhalten.",
        it: "Fonte non disponibile. Il collegamento di provenienza è stato conservato.",
        ja: "ソースを利用できません。出どころへのリンクは保持されています。",
        zh: "来源不可用。出处关联已保留。",
        ko: "출처를 사용할 수 없어요. 출처 연결은 보존되었어요.",
        ru: "Источник недоступен. Ссылка на происхождение сохранена." }
    /// Title of the export preview.
    export_preview_title { en: "Export preview", pt: "Prévia da exportação",
        es: "Vista previa de la exportación", fr: "Aperçu de l’export",
        de: "Exportvorschau", it: "Anteprima dell’esportazione",
        ja: "エクスポートのプレビュー", zh: "导出预览", ko: "내보내기 미리 보기",
        ru: "Предпросмотр экспорта" }
    /// Button that opens the file dialog of the export.
    choose_destination { en: "Choose destination…", pt: "Escolher destino…",
        es: "Elegir destino…", fr: "Choisir la destination…", de: "Ziel wählen…",
        it: "Scegli la destinazione…", ja: "保存先を選ぶ…", zh: "选择位置…",
        ko: "위치 선택…", ru: "Выбрать место…" }
    /// Button that confirms replacing an existing export file.
    overwrite_button { en: "Replace file", pt: "Substituir arquivo",
        es: "Reemplazar archivo", fr: "Remplacer le fichier", de: "Datei ersetzen",
        it: "Sostituisci file", ja: "ファイルを置き換える", zh: "替换文件",
        ko: "파일 바꾸기", ru: "Заменить файл" }
    /// Retry button of the error banner.
    retry { en: "Try again", pt: "Tentar novamente", es: "Reintentar", fr: "Réessayer",
        de: "Erneut versuchen", it: "Riprova", ja: "再試行", zh: "重试", ko: "다시 시도",
        ru: "Повторить" }
    /// Error: a decision could not be read.
    open_failed { en: "Couldn't open this decision.",
        pt: "Não foi possível abrir esta decisão.",
        es: "No se pudo abrir esta decisión.", fr: "Impossible d’ouvrir cette décision.",
        de: "Diese Entscheidung konnte nicht geöffnet werden.",
        it: "Impossibile aprire questa decisione.",
        ja: "この決定を開けませんでした。", zh: "无法打开此决策。",
        ko: "이 결정을 열지 못했어요.", ru: "Не удалось открыть это решение." }
    /// Error: the decision belongs to another project.
    not_in_project { en: "This decision doesn't belong to the selected project.",
        pt: "Esta decisão não pertence ao projeto selecionado.",
        es: "Esta decisión no pertenece al proyecto seleccionado.",
        fr: "Cette décision n’appartient pas au projet sélectionné.",
        de: "Diese Entscheidung gehört nicht zum ausgewählten Projekt.",
        it: "Questa decisione non appartiene al progetto selezionato.",
        ja: "この決定は選択中のプロジェクトのものではありません。",
        zh: "此决策不属于所选项目。", ko: "이 결정은 선택한 프로젝트에 속하지 않아요.",
        ru: "Это решение не относится к выбранному проекту." }
    /// Error: the sources of a decision could not be read.
    sources_failed { en: "Couldn't load this decision's sources.",
        pt: "Não foi possível carregar as fontes desta decisão.",
        es: "No se pudieron cargar las fuentes de esta decisión.",
        fr: "Impossible de charger les sources de cette décision.",
        de: "Die Quellen dieser Entscheidung konnten nicht geladen werden.",
        it: "Impossibile caricare le fonti di questa decisione.",
        ja: "この決定のソースを読み込めませんでした。", zh: "无法加载此决策的来源。",
        ko: "이 결정의 출처를 불러오지 못했어요.",
        ru: "Не удалось загрузить источники этого решения." }
    /// Month heading of a row whose date is not valid.
    date_unregistered { en: "Date not recorded", pt: "Data não registrada",
        es: "Fecha no registrada", fr: "Date non enregistrée", de: "Datum nicht erfasst",
        it: "Data non registrata", ja: "日付は記録されていません", zh: "未记录日期",
        ko: "날짜가 기록되지 않았어요", ru: "Дата не записана" }
    /// Toast: a decision was superseded.
    notice_superseded { en: "Decision superseded. The previous one stays in history.",
        pt: "Decisão substituída. A anterior segue no histórico.",
        es: "Decisión sustituida. La anterior sigue en el historial.",
        fr: "Décision remplacée. La précédente reste dans l’historique.",
        de: "Entscheidung ersetzt. Die vorherige bleibt im Verlauf.",
        it: "Decisione sostituita. La precedente resta nella cronologia.",
        ja: "決定を置き換えました。以前の決定は履歴に残ります。",
        zh: "决策已被取代。之前的决策保留在历史中。",
        ko: "결정을 대체했어요. 이전 결정은 기록에 남아요.",
        ru: "Решение заменено. Прежнее остаётся в истории." }
    /// Toast: a dependency was recorded.
    notice_depends { en: "Dependency recorded.", pt: "Dependência registrada.",
        es: "Dependencia registrada.", fr: "Dépendance enregistrée.",
        de: "Abhängigkeit erfasst.", it: "Dipendenza registrata.",
        ja: "依存関係を記録しました。", zh: "已记录依赖关系。",
        ko: "의존 관계를 기록했어요.", ru: "Зависимость записана." }
    /// Toast: a conflict was recorded.
    notice_conflict { en: "Conflict recorded on both decisions.",
        pt: "Conflito registrado nas duas decisões.",
        es: "Conflicto registrado en las dos decisiones.",
        fr: "Conflit enregistré sur les deux décisions.",
        de: "Konflikt bei beiden Entscheidungen erfasst.",
        it: "Conflitto registrato su entrambe le decisioni.",
        ja: "両方の決定に競合を記録しました。", zh: "已在两项决策上记录冲突。",
        ko: "두 결정 모두에 충돌을 기록했어요.", ru: "Конфликт записан в обоих решениях." }
    /// Title of the revision form.
    editor_title { en: "Revise decision", pt: "Revisar decisão", es: "Revisar decisión",
        fr: "Réviser la décision", de: "Entscheidung überarbeiten",
        it: "Rivedi la decisione", ja: "決定を改訂", zh: "修订决策", ko: "결정 수정",
        ru: "Пересмотр решения" }
    /// Note under the title of the revision form.
    editor_history_note { en: "The previous version stays in history.",
        pt: "A versão anterior permanece no histórico.",
        es: "La versión anterior permanece en el historial.",
        fr: "La version précédente reste dans l’historique.",
        de: "Die vorherige Version bleibt im Verlauf.",
        it: "La versione precedente resta nella cronologia.",
        ja: "以前のバージョンは履歴に残ります。", zh: "上一版本保留在历史中。",
        ko: "이전 버전은 기록에 남아요.", ru: "Предыдущая версия остаётся в истории." }
    /// Hint of the list fields of the revision form.
    one_per_line { en: "One item per line.", pt: "Um item por linha.",
        es: "Un elemento por línea.", fr: "Un élément par ligne.",
        de: "Ein Eintrag pro Zeile.", it: "Una voce per riga.", ja: "1行につき1項目。",
        zh: "每行一项。", ko: "한 줄에 한 항목씩 적어요.", ru: "Один пункт на строку." }
    /// Footer message while the revision is saving.
    saving { en: "Saving…", pt: "Salvando…", es: "Guardando…", fr: "Enregistrement…",
        de: "Wird gespeichert…", it: "Salvataggio…", ja: "保存中…", zh: "正在保存…",
        ko: "저장 중…", ru: "Сохранение…" }
    /// Footer message when the revision form is invalid.
    fields_invalid { en: "Fill in the required fields and respect the length limits.",
        pt: "Preencha os campos obrigatórios e respeite os limites de tamanho.",
        es: "Completa los campos obligatorios y respeta los límites de tamaño.",
        fr: "Renseignez les champs obligatoires et respectez les limites de longueur.",
        de: "Fülle die Pflichtfelder aus und beachte die Längenbegrenzungen.",
        it: "Compila i campi obbligatori e rispetta i limiti di lunghezza.",
        ja: "必須項目を入力し、文字数の上限を守ってください。",
        zh: "请填写必填项，并遵守长度限制。",
        ko: "필수 항목을 채우고 길이 제한을 지켜 주세요.",
        ru: "Заполните обязательные поля и соблюдайте ограничения по длине." }
    /// Footer message when the revision has no change.
    no_changes { en: "No changes yet.", pt: "Nenhuma alteração ainda.",
        es: "Aún no hay cambios.", fr: "Aucune modification pour l’instant.",
        de: "Noch keine Änderungen.", it: "Ancora nessuna modifica.",
        ja: "まだ変更はありません。", zh: "尚无更改。", ko: "아직 변경 사항이 없어요.",
        ru: "Изменений пока нет." }
    /// Button and accessible name that save the revision.
    save_version { en: "Save new version", pt: "Salvar nova versão",
        es: "Guardar nueva versión", fr: "Enregistrer la nouvelle version",
        de: "Neue Version speichern", it: "Salva nuova versione",
        ja: "新しいバージョンを保存", zh: "保存新版本", ko: "새 버전 저장",
        ru: "Сохранить новую версию" }
    /// Accessible name of the cancel button of the revision form.
    cancel_revision_aria { en: "Cancel revision", pt: "Cancelar revisão",
        es: "Cancelar revisión", fr: "Annuler la révision", de: "Überarbeitung abbrechen",
        it: "Annulla la revisione", ja: "改訂をキャンセル", zh: "取消修订",
        ko: "수정 취소", ru: "Отменить пересмотр" }
}

formats! {
    /// Toast: the export was written.
    exported(path: &str) { en: "Exported to {path}", pt: "Exportado para {path}",
        es: "Exportado a {path}", fr: "Exporté vers {path}", de: "Exportiert nach {path}",
        it: "Esportato in {path}", ja: "{path} にエクスポートしました",
        zh: "已导出到 {path}", ko: "{path}에 내보냈어요", ru: "Экспортировано в {path}" }
    /// Error: the relation is not allowed; `reason` comes from the backend.
    relation_not_allowed(reason: &str) {
        en: "Relation not allowed: {reason}.", pt: "Relação não permitida: {reason}.",
        es: "Relación no permitida: {reason}.", fr: "Relation non autorisée : {reason}.",
        de: "Beziehung nicht erlaubt: {reason}.", it: "Relazione non consentita: {reason}.",
        ja: "許可されない関係です: {reason}。", zh: "不允许的关系：{reason}。",
        ko: "허용되지 않는 관계예요: {reason}.", ru: "Связь не разрешена: {reason}." }
    /// Footer of the index while a part is chosen.
    part_summary(count: usize, total: usize, name: &str) {
        en: "{count} of {total} in effect · {name}", pt: "{count} de {total} em vigor · {name}",
        es: "{count} de {total} vigentes · {name}", fr: "{count} sur {total} en vigueur · {name}",
        de: "{count} von {total} in Kraft · {name}", it: "{count} di {total} in vigore · {name}",
        ja: "{total} 件中 {count} 件が有効 · {name}", zh: "生效中 {count}/{total} · {name}",
        ko: "유효 {count}/{total} · {name}", ru: "Действует: {count} из {total} · {name}" }
    /// Footer of the index listing confirmed decisions.
    loaded_summary(count: usize) {
        en: "{count} loaded · confirmation order", pt: "{count} carregadas · ordem de confirmação",
        es: "{count} cargadas · orden de confirmación",
        fr: "{count} chargées · ordre de confirmation",
        de: "{count} geladen · Reihenfolge der Bestätigung",
        it: "{count} caricate · ordine di conferma",
        ja: "{count} 件読み込み済み · 確定順", zh: "已加载 {count} 条 · 按确认顺序",
        ko: "{count}개 불러옴 · 확인 순서", ru: "Загружено: {count} · в порядке подтверждения" }
    /// Accessible name of the control that clears the part filter.
    clear_part_aria(name: &str) {
        en: "Show all parts, leave {name}", pt: "Mostrar todas as partes, sair de {name}",
        es: "Mostrar todas las partes, salir de {name}",
        fr: "Afficher toutes les parties, quitter {name}",
        de: "Alle Teile anzeigen, {name} verlassen",
        it: "Mostra tutte le parti, esci da {name}",
        ja: "すべてのパートを表示、{name} を解除", zh: "显示所有部分，退出 {name}",
        ko: "모든 파트 보기, {name} 해제", ru: "Показать все части, выйти из {name}" }
    /// Empty index: a part with no decision in effect.
    empty_index_part(name: &str) {
        en: "No decision in effect about {name}.",
        pt: "Nenhuma decisão em vigor sobre {name}.",
        es: "Ninguna decisión vigente sobre {name}.",
        fr: "Aucune décision en vigueur sur {name}.",
        de: "Keine Entscheidung in Kraft zu {name}.",
        it: "Nessuna decisione in vigore su {name}.",
        ja: "{name} に関する有効な決定はありません。", zh: "没有关于 {name} 的生效决策。",
        ko: "{name}에 대한 유효한 결정이 없어요.", ru: "Нет действующих решений о {name}." }
    /// Empty index: a search with no result inside a part.
    empty_search_part(name: &str) {
        en: "No results in {name}. Try another word.",
        pt: "Nenhum resultado em {name}. Tente outra palavra.",
        es: "Sin resultados en {name}. Prueba otra palabra.",
        fr: "Aucun résultat dans {name}. Essayez un autre mot.",
        de: "Keine Treffer in {name}. Probiere ein anderes Wort.",
        it: "Nessun risultato in {name}. Prova un’altra parola.",
        ja: "{name} に結果はありません。別の言葉をお試しください。",
        zh: "{name} 中没有结果。试试其他词。",
        ko: "{name}에서 결과가 없어요. 다른 단어를 입력해 보세요.",
        ru: "В {name} ничего не найдено. Попробуйте другое слово." }
    /// Title of the empty panel of a part when a search finds nothing.
    part_search_empty_title(name: &str) {
        en: "No {name} decision matches this search",
        pt: "Nenhuma decisão de {name} com essa busca",
        es: "Ninguna decisión de {name} coincide con esta búsqueda",
        fr: "Aucune décision sur {name} ne correspond à cette recherche",
        de: "Keine Entscheidung zu {name} passt zu dieser Suche",
        it: "Nessuna decisione su {name} corrisponde a questa ricerca",
        ja: "この検索に一致する {name} の決定はありません",
        zh: "没有与此搜索匹配的 {name} 决策",
        ko: "이 검색과 일치하는 {name} 결정이 없어요",
        ru: "Нет решений о {name} по этому запросу" }
    /// Body of the empty panel of a part when a search finds nothing.
    part_search_empty_body(name: &str) {
        en: "Search covers the question, choice and rationale of the decisions in effect about {name}. Try another word or all parts.",
        pt: "A busca cobre pergunta, escolha e justificativa das decisões em vigor sobre {name}. Tente outra palavra ou todas as partes.",
        es: "La búsqueda abarca la pregunta, la elección y la justificación de las decisiones vigentes sobre {name}. Prueba otra palabra o todas las partes.",
        fr: "La recherche porte sur la question, le choix et la justification des décisions en vigueur sur {name}. Essayez un autre mot ou toutes les parties.",
        de: "Die Suche umfasst Frage, Wahl und Begründung der Entscheidungen in Kraft zu {name}. Probiere ein anderes Wort oder alle Teile.",
        it: "La ricerca copre domanda, scelta e motivazione delle decisioni in vigore su {name}. Prova un’altra parola o tutte le parti.",
        ja: "検索対象は、{name} に関する有効な決定の質問・選択・判断理由です。別の言葉を試すか、すべてのパートを表示してください。",
        zh: "搜索范围包括 {name} 相关生效决策的问题、选择和理由。试试其他词，或查看所有部分。",
        ko: "{name}에 대한 유효한 결정의 질문, 선택, 판단 이유를 검색해요. 다른 단어를 입력하거나 모든 파트를 보세요.",
        ru: "Поиск охватывает вопрос, выбор и обоснование действующих решений о {name}. Попробуйте другое слово или все части." }
    /// Title of the empty panel of a part with no decision.
    part_empty_title(name: &str) {
        en: "Nothing decided about {name} yet", pt: "Nada decidido sobre {name} ainda",
        es: "Aún no hay nada decidido sobre {name}",
        fr: "Rien de décidé sur {name} pour l’instant",
        de: "Zu {name} wurde noch nichts entschieden",
        it: "Ancora nulla di deciso su {name}",
        ja: "{name} について決定したことはまだありません",
        zh: "尚未就 {name} 做出决策", ko: "{name}에 대해 아직 결정한 것이 없어요",
        ru: "О {name} пока ничего не решено" }
    /// Body of the empty panel of a part with no decision.
    part_empty_body(name: &str) {
        en: "A decision appears here when it is linked to {name}, or to a part of it, on the Map. Confirm the suggested links in Review or link a decision on the component’s page.",
        pt: "Uma decisão aparece aqui quando está ligada a {name}, ou a uma parte dela, no Mapa. Confirme as ligações sugeridas na Revisão ou vincule uma decisão na página do componente.",
        es: "Una decisión aparece aquí cuando está vinculada a {name}, o a alguna de sus partes, en el Mapa. Confirma los vínculos sugeridos en Revisión o vincula una decisión en la página del componente.",
        fr: "Une décision apparaît ici lorsqu’elle est liée à {name}, ou à l’une de ses parties, dans la Carte. Confirmez les liens suggérés dans Revue ou liez une décision sur la page du composant.",
        de: "Eine Entscheidung erscheint hier, wenn sie in der Karte mit {name} oder einem Teil davon verknüpft ist. Bestätige die vorgeschlagenen Verknüpfungen in der Prüfung oder verknüpfe eine Entscheidung auf der Seite der Komponente.",
        it: "Una decisione compare qui quando è collegata a {name}, o a una sua parte, nella Mappa. Conferma i collegamenti suggeriti in Revisione o collega una decisione nella pagina del componente.",
        ja: "決定は、マップで {name} またはその一部にリンクされるとここに表示されます。レビューで提案されたリンクを確定するか、コンポーネントのページで決定をリンクしてください。",
        zh: "当决策在地图中关联到 {name} 或其某个部分时，会显示在这里。请在审阅中确认建议的关联，或在组件页面中关联决策。",
        ko: "맵에서 {name} 또는 그 일부에 연결된 결정이 여기에 표시돼요. 검토에서 제안된 연결을 확인하거나 컴포넌트 페이지에서 결정을 연결해 보세요.",
        ru: "Решение появляется здесь, когда оно связано на карте с {name} или его частью. Подтвердите предложенные ссылки в проверке или свяжите решение на странице компонента." }
    /// Button of the current version in the history.
    version_line_current(version: i64, date: &str) {
        en: "v{version} · {date} · current", pt: "v{version} · {date} · atual",
        es: "v{version} · {date} · actual", fr: "v{version} · {date} · actuelle",
        de: "v{version} · {date} · aktuell", it: "v{version} · {date} · attuale",
        ja: "v{version} · {date} · 現在", zh: "v{version} · {date} · 当前",
        ko: "v{version} · {date} · 현재", ru: "v{version} · {date} · текущая" }
    /// Accessible name of a candidate decision to relate to.
    relate_with_aria(question: &str) {
        en: "Relate with: {question}", pt: "Relacionar com: {question}",
        es: "Relacionar con: {question}", fr: "Relier à : {question}",
        de: "Verknüpfen mit: {question}", it: "Collega con: {question}",
        ja: "{question} と関連付ける", zh: "与此关联：{question}",
        ko: "관계 추가: {question}", ru: "Связать с: {question}" }
    /// Size line of the export preview.
    export_size(bytes: usize) {
        en: "{bytes} bytes · exact content of the current version",
        pt: "{bytes} bytes · conteúdo exato da versão atual",
        es: "{bytes} bytes · contenido exacto de la versión actual",
        fr: "{bytes} octets · contenu exact de la version actuelle",
        de: "{bytes} Bytes · genauer Inhalt der aktuellen Version",
        it: "{bytes} byte · contenuto esatto della versione attuale",
        ja: "{bytes} バイト · 現在のバージョンの内容そのまま",
        zh: "{bytes} 字节 · 当前版本的完整内容",
        ko: "{bytes}바이트 · 현재 버전의 내용 그대로",
        ru: "{bytes} байт · точное содержимое текущей версии" }
    /// Question before replacing an existing export file.
    overwrite_prompt(path: &str) {
        en: "The file {path} already exists. Replace its contents?",
        pt: "O arquivo {path} já existe. Substituir seu conteúdo?",
        es: "El archivo {path} ya existe. ¿Reemplazar su contenido?",
        fr: "Le fichier {path} existe déjà. Remplacer son contenu ?",
        de: "Die Datei {path} existiert bereits. Inhalt ersetzen?",
        it: "Il file {path} esiste già. Sostituirne il contenuto?",
        ja: "ファイル {path} はすでにあります。内容を置き換えますか？",
        zh: "文件 {path} 已存在。要替换其内容吗？",
        ko: "{path} 파일이 이미 있어요. 내용을 바꿀까요?",
        ru: "Файл {path} уже существует. Заменить его содержимое?" }
}

/// Footer of the index while searching.
pub fn results_summary(count: usize) -> String {
    use super::Language::*;
    let one = super::one(count);
    match super::current() {
        English => format!(
            "{count} {} in question, choice and rationale",
            if one { "result" } else { "results" }
        ),
        Portuguese => format!(
            "{count} {} em pergunta, escolha e justificativa",
            if one { "resultado" } else { "resultados" }
        ),
        Spanish => format!(
            "{count} {} en pregunta, elección y justificación",
            if one { "resultado" } else { "resultados" }
        ),
        French => format!(
            "{count} {} dans la question, le choix et la justification",
            if super::french_one(count) {
                "résultat"
            } else {
                "résultats"
            }
        ),
        German => format!("{count} Treffer in Frage, Wahl und Begründung"),
        Italian => format!(
            "{count} {} in domanda, scelta e motivazione",
            if one { "risultato" } else { "risultati" }
        ),
        Japanese => format!("質問・選択・判断理由の検索結果: {count} 件"),
        Chinese => format!("在问题、选择和理由中找到 {count} 条结果"),
        Korean => format!("질문, 선택, 판단 이유에서 {count}건 찾았어요"),
        Russian => format!(
            "{count} {} в вопросе, выборе и обосновании",
            ["результат", "результата", "результатов"][super::russian_form(count)]
        ),
    }
}

/// Button that opens the version history, with the number of versions.
pub fn history_label(count: usize) -> String {
    use super::Language::*;
    let many = usize::from(!super::one(count));
    let word = match super::current() {
        English => ["Version", "Versions"][many],
        Portuguese => ["Versão", "Versões"][many],
        Spanish => ["Versión", "Versiones"][many],
        French => ["Version", "Versions"][usize::from(!super::french_one(count))],
        German => ["Version", "Versionen"][many],
        Italian => ["Versione", "Versioni"][many],
        Japanese => "バージョン",
        Chinese => "版本",
        Korean => "버전",
        Russian => ["Версия", "Версии", "Версий"][super::russian_form(count)],
    };
    format!("{word} · {count}")
}

/// The count of sources in the header of the evidence.
pub fn sources_count(count: usize) -> String {
    use super::Language::*;
    let many = usize::from(!super::one(count));
    match super::current() {
        English => format!("{count} {}", ["source", "sources"][many]),
        Portuguese => format!("{count} {}", ["fonte", "fontes"][many]),
        Spanish => format!("{count} {}", ["fuente", "fuentes"][many]),
        French => format!(
            "{count} {}",
            ["source", "sources"][usize::from(!super::french_one(count))]
        ),
        German => format!("{count} {}", ["Quelle", "Quellen"][many]),
        Italian => format!("{count} {}", ["fonte", "fonti"][many]),
        Japanese => format!("{count} 件のソース"),
        Chinese => format!("{count} 个来源"),
        Korean => format!("출처 {count}개"),
        Russian => format!(
            "{count} {}",
            ["источник", "источника", "источников"][super::russian_form(count)]
        ),
    }
}

/// Heading of a month in the index: `September 2026`, `2026年9月`.
pub fn month_year(month0: u32, year: i32) -> String {
    use super::Language::*;
    let month = month0.min(11) as usize;
    let number = month + 1;
    let name = match super::current() {
        English => [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ][month],
        Portuguese => [
            "Janeiro",
            "Fevereiro",
            "Março",
            "Abril",
            "Maio",
            "Junho",
            "Julho",
            "Agosto",
            "Setembro",
            "Outubro",
            "Novembro",
            "Dezembro",
        ][month],
        Spanish => [
            "Enero",
            "Febrero",
            "Marzo",
            "Abril",
            "Mayo",
            "Junio",
            "Julio",
            "Agosto",
            "Septiembre",
            "Octubre",
            "Noviembre",
            "Diciembre",
        ][month],
        French => [
            "Janvier",
            "Février",
            "Mars",
            "Avril",
            "Mai",
            "Juin",
            "Juillet",
            "Août",
            "Septembre",
            "Octobre",
            "Novembre",
            "Décembre",
        ][month],
        German => [
            "Januar",
            "Februar",
            "März",
            "April",
            "Mai",
            "Juni",
            "Juli",
            "August",
            "September",
            "Oktober",
            "November",
            "Dezember",
        ][month],
        Italian => [
            "Gennaio",
            "Febbraio",
            "Marzo",
            "Aprile",
            "Maggio",
            "Giugno",
            "Luglio",
            "Agosto",
            "Settembre",
            "Ottobre",
            "Novembre",
            "Dicembre",
        ][month],
        Russian => [
            "Январь",
            "Февраль",
            "Март",
            "Апрель",
            "Май",
            "Июнь",
            "Июль",
            "Август",
            "Сентябрь",
            "Октябрь",
            "Ноябрь",
            "Декабрь",
        ][month],
        Japanese | Chinese => return format!("{year}年{number}月"),
        Korean => return format!("{year}년 {number}월"),
    };
    format!("{name} {year}")
}
