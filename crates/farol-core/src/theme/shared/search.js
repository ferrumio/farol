// Client-side search. Loads the pre-built index on demand and does
// token lookups against scores computed at build time.

(function () {
  var MODAL_ID = 'farol-search-modal';
  var state = { index: null, docs: null, loading: false, ready: false };

  function open() {
    var modal = document.getElementById(MODAL_ID);
    if (!modal) return;
    modal.classList.add('open');
    var input = modal.querySelector('input');
    if (input) { input.focus(); input.select(); }
    ensureLoaded();
  }
  function close() {
    var modal = document.getElementById(MODAL_ID);
    if (modal) modal.classList.remove('open');
  }

  function ensureLoaded() {
    if (state.ready || state.loading) return;
    state.loading = true;
    Promise.all([
      fetch('/assets/search/docs.json').then(function (r) { return r.json(); }),
      fetch('/assets/search/index.json').then(function (r) { return r.json(); }),
    ]).then(function (results) {
      state.docs = results[0];
      state.index = results[1].index;
      state.ready = true;
    }).catch(function (err) {
      console.error('farol search: failed to load index', err);
    }).finally(function () { state.loading = false; });
  }

  // Very small stemmer: lowercase + stripped common English suffixes.
  // Matches the tantivy-side English tokenizer well enough for common terms.
  function tokenize(query) {
    return query.toLowerCase()
      .split(/[^a-z0-9_]+/)
      .filter(Boolean)
      .map(stem);
  }
  function stem(word) {
    // English snowball-lite.
    if (word.length < 4) return word;
    var suffixes = ['ingly', 'edly', 'ings', 'ing', 'ed', 'es', 's', 'ly'];
    for (var i = 0; i < suffixes.length; i++) {
      var s = suffixes[i];
      if (word.endsWith(s) && word.length - s.length >= 3) {
        return word.slice(0, -s.length);
      }
    }
    return word;
  }

  function search(query) {
    if (!state.ready) return [];
    var tokens = tokenize(query);
    if (tokens.length === 0) return [];

    var scores = {};
    for (var i = 0; i < tokens.length; i++) {
      var postings = state.index[tokens[i]];
      if (!postings) {
        // prefix fallback: try first token starting with it.
        postings = prefixLookup(tokens[i]);
      }
      if (!postings) continue;
      for (var j = 0; j < postings.length; j++) {
        var p = postings[j];
        scores[p.doc] = (scores[p.doc] || 0) + p.score;
      }
    }

    var results = [];
    for (var id in scores) {
      results.push({ doc: state.docs[+id], score: scores[id] });
    }
    results.sort(function (a, b) { return b.score - a.score; });
    return results.slice(0, 20);
  }

  function prefixLookup(token) {
    // Scan keys cheaply; O(V) per miss but V is small on real sites.
    var keys = Object.keys(state.index);
    for (var i = 0; i < keys.length; i++) {
      if (keys[i].length > token.length && keys[i].indexOf(token) === 0) {
        return state.index[keys[i]];
      }
    }
    return null;
  }

  function highlight(text, tokens) {
    if (!tokens.length) return escapeHtml(text);
    var lower = text.toLowerCase();
    var parts = [];
    var cursor = 0;
    while (cursor < text.length) {
      var nextStart = -1, nextLen = 0;
      for (var i = 0; i < tokens.length; i++) {
        var idx = lower.indexOf(tokens[i], cursor);
        if (idx !== -1 && (nextStart === -1 || idx < nextStart)) {
          nextStart = idx;
          nextLen = tokens[i].length;
        }
      }
      if (nextStart === -1) {
        parts.push(escapeHtml(text.slice(cursor)));
        break;
      }
      parts.push(escapeHtml(text.slice(cursor, nextStart)));
      parts.push('<mark>');
      parts.push(escapeHtml(text.slice(nextStart, nextStart + nextLen)));
      parts.push('</mark>');
      cursor = nextStart + nextLen;
    }
    return parts.join('');
  }
  function escapeHtml(s) {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }

  function render(results, tokens) {
    var list = document.getElementById('farol-search-results');
    if (!list) return;
    list.innerHTML = '';
    if (results.length === 0) {
      list.innerHTML = '<li class="empty">No results.</li>';
      return;
    }
    for (var i = 0; i < results.length; i++) {
      var r = results[i];
      var li = document.createElement('li');
      li.className = 'result' + (i === 0 ? ' active' : '');
      li.innerHTML =
        '<a href="' + escapeHtml(r.doc.url) + '">' +
        '<span class="title">' + highlight(r.doc.title, tokens) + '</span>' +
        '<span class="snippet">' + highlight(r.doc.snippet, tokens) + '</span>' +
        '</a>';
      list.appendChild(li);
    }
  }

  function moveActive(direction) {
    var items = document.querySelectorAll('#farol-search-results .result');
    if (items.length === 0) return;
    var currentIdx = -1;
    for (var i = 0; i < items.length; i++) {
      if (items[i].classList.contains('active')) { currentIdx = i; break; }
    }
    if (currentIdx >= 0) items[currentIdx].classList.remove('active');
    var next = (currentIdx + direction + items.length) % items.length;
    items[next].classList.add('active');
    items[next].scrollIntoView({ block: 'nearest' });
  }

  function gotoActive() {
    var active = document.querySelector('#farol-search-results .result.active a');
    if (active) window.location.href = active.getAttribute('href');
  }

  // Wire up shortcuts.
  document.addEventListener('keydown', function (e) {
    var isMac = navigator.platform.toUpperCase().indexOf('MAC') >= 0;
    var modifier = isMac ? e.metaKey : e.ctrlKey;
    if (modifier && e.key === 'k') {
      e.preventDefault();
      open();
      return;
    }
    if (e.key === 'Escape') {
      close();
      return;
    }
    var modal = document.getElementById(MODAL_ID);
    if (!modal || !modal.classList.contains('open')) return;
    if (e.key === 'ArrowDown') { e.preventDefault(); moveActive(1); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); moveActive(-1); }
    else if (e.key === 'Enter') { e.preventDefault(); gotoActive(); }
  });

  document.addEventListener('click', function (e) {
    var trigger = e.target.closest('[data-farol-search-trigger]');
    if (trigger) { e.preventDefault(); open(); return; }
    var backdrop = e.target.closest('.farol-search-backdrop');
    if (backdrop && e.target === backdrop) { close(); }
  });

  document.addEventListener('input', function (e) {
    if (e.target.id !== 'farol-search-input') return;
    var tokens = tokenize(e.target.value);
    var results = search(e.target.value);
    render(results, tokens);
  });
})();
