// Presentation and reversible interactions for the selected study. No storage.
const cIcons = {
  copy: '<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V4H4v12h4"/></svg>',
  expand: '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M14 4h6v6M10 20H4v-6M20 4l-7 7M4 20l7-7"/></svg>',
  link: '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="m10 13 4-4M8 15l-2 2a4 4 0 0 1-6-6l5-5a4 4 0 0 1 6 0M16 9l2-2a4 4 0 0 1 6 6l-5 5a4 4 0 0 1-6 0" transform="translate(1 0) scale(.9)"/></svg>',
  export: '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 15V3m-4 4 4-4 4 4M5 13v7h14v-7"/></svg>'
};
function indexC(items) {
  const groups = new Map();
  items.forEach(d => { const key = d.date.split(' ').slice(1).join(' '); if (!groups.has(key)) groups.set(key, []); groups.get(key).push(d); });
  const labels = { 'set 2026': 'Setembro 2026' };
  return `<aside class="index" aria-label="Índice temporal de decisões"><header class="c-index-head"><div class="c-index-title"><h2>Índice de decisões</h2><span>${items.length} ${query?'resultados':'carregadas'}</span></div><p class="c-index-subtitle">Escolhas preservadas neste projeto</p>${controls()}${query?'<p class="c-index-scope">Busca no projeto · até 50 resultados</p>':''}</header>${items.length ? [...groups].map(([key, rows]) => `<div class="group-label">${esc(labels[key] || key).toLocaleUpperCase()}</div>${entries(rows)}`).join('') : '<div class="empty"><strong>Nenhuma decisão encontrada</strong>Tente outro termo ou estado.</div>'}<div class="c-index-tail">Selecione uma decisão para ler.<br>As revisões anteriores ficam no histórico.</div></aside>`;
}
function sourceContentC(d) { return source === 0 ? d.code : 'Registro fictício da captura.\nA escolha foi discutida na sessão e confirmada após revisão humana.'; }
function docC(d) {
  const old = detailTab === 'history' && revision > 0;
  const v = old ? Math.max(1, d.v - revision) : d.v;
  const choice = old && d.id === 1 ? 'Validar o candidato antes de persistir a decisão.' : d.choice;
  const lines = sourceContentC(d).split('\n');
  return `<div class="document">${old?'<div class="history-note">Versão anterior · somente leitura. Revisar e exportar usam a versão atual.</div>':''}<div class="c-document-meta">${badge(d)}<span>v${v}${old?'':' · atual'}</span><span>·</span><span>${d.date}</span></div><h2>${esc(d.q)}</h2><section class="c-choice"><h3>Escolha confirmada</h3><p class="choice">${esc(choice)}</p></section><section class="c-rationale"><h3>Justificativa</h3><p>${esc(d.rationale)}</p></section><section class="c-context"><h3>Contexto da decisão</h3><div class="c-context-grid">${[['Escopo',d.scope],['Premissas',d.assumption],['Consequências',d.consequence],['Reconsiderar quando',d.reconsider]].map(([name, text])=>`<details><summary>${name}</summary><ul><li>${esc(text)}</li></ul></details>`).join('')}</div></section><section><div class="c-section-heading"><h3>Evidências</h3><span class="muted">2 fontes</span></div><div class="code-panel"><div class="files" aria-label="Fontes da decisão"><button data-source="0" class="${source===0?'active':''}" aria-pressed="${source===0}">${fileIcon}${esc(d.source)}</button><button data-source="1" class="${source===1?'active':''}" aria-pressed="${source===1}">${fileIcon}captura.txt</button></div><div class="c-source-bar"><div><strong>${source===0?esc(d.path):'captura.txt'}</strong><span>${source===0?`Trecho de alteração · linhas 24–${23+lines.length}`:'Texto da captura · origem preservada'}</span></div><div class="c-source-tools"><button data-action="copy-source" aria-label="Copiar trecho">${cIcons.copy}<span id="copy-source-label">Copiar trecho</span></button><button data-action="expand-source" aria-label="Expandir leitura">${cIcons.expand}</button></div></div><pre class="${source===1?'c-prose':''}" aria-label="Conteúdo da fonte">${lines.map((line,i)=>`<span class="ln" aria-hidden="true">${source===0?24+i:1+i}</span>${esc(line)}`).join('\n')}</pre></div><div class="source-footer">${cIcons.link}<span>Captura local</span><span>·</span><span>sessão opencode-demo-42</span><span id="copy-source-feedback" role="status"></span></div></section></div>`;
}
function readerC(d) {
  if (!d) return '<article class="reader"><div class="empty"><strong>Encontre uma decisão no índice</strong>O documento e suas fontes aparecem aqui.</div></article>';
  return `<article class="reader"><header class="reader-toolbar"><nav class="c-reading-tabs" aria-label="Leitura"><button data-detail="document" class="${detailTab==='document'?'active':''}" aria-pressed="${detailTab==='document'}">Documento</button><button data-detail="history" class="${detailTab==='history'?'active':''}" aria-pressed="${detailTab==='history'}">Histórico <span>${d.v}</span></button></nav><div class="actions"><button data-action="export">${cIcons.export}Exportar…</button>${d.status==='Confirmada'?'<button data-action="revise" class="primary" aria-label="Revisar">Revisar</button>':''}</div></header><div class="reader-scroll">${detailTab==='history'?`<div class="history"><aside class="revision-index" aria-label="Versões da decisão">${Array.from({length:d.v},(_,i)=>`<button data-revision="${i}" class="${revision===i?'active':''}" aria-pressed="${revision===i}">v${d.v-i}${i===0?' · atual':''}</button>`).join('')}</aside><div>${docC(d)}</div></div>`:docC(d)}</div></article>`;
}
function sourceActionC(kind) {
  const d = data.find(d => d.id === selected);
  if (kind === 'copy-source') {
    const feedback = document.getElementById('copy-source-feedback');
    navigator.clipboard.writeText(sourceContentC(d)).then(() => {
      const label = document.getElementById('copy-source-label');
      if (label) label.textContent = 'Copiado';
      if (feedback) feedback.textContent = '· Trecho copiado';
    }).catch(() => { if (feedback) feedback.textContent = '· Não foi possível copiar. Selecione o trecho.'; });
    return true;
  }
  if (kind === 'expand-source') {
    modal(source === 0 ? d.path : 'captura.txt', `<p class="muted small">Fonte da decisão · leitura ampliada</p><pre class="c-expanded">${esc(sourceContentC(d))}</pre>`);
    return true;
  }
  return false;
}
