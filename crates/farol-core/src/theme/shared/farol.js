(function () {
  // ------------------------------------------------------------------ Theme
  // Restore stored preference on first paint (inlined as <script> for a flash-free swap).
  var THEME_KEY = 'farol-theme';
  try {
    var stored = localStorage.getItem(THEME_KEY);
    if (stored === 'dark' || stored === 'light') {
      document.documentElement.setAttribute('data-theme', stored);
    }
  } catch (_) { /* private mode, localStorage off */ }

  function currentTheme() {
    var attr = document.documentElement.getAttribute('data-theme');
    if (attr === 'dark' || attr === 'light') return attr;
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }
  function toggleTheme() {
    var next = currentTheme() === 'dark' ? 'light' : 'dark';
    document.documentElement.setAttribute('data-theme', next);
    try { localStorage.setItem(THEME_KEY, next); } catch (_) {}
  }

  // ------------------------------------------------------------------ Sidebar drawer
  function sidebarOpen(open) {
    var bar = document.querySelector('[data-sidebar]');
    var back = document.querySelector('[data-sidebar-backdrop]');
    if (!bar) return;
    if (open === undefined) open = !bar.classList.contains('open');
    bar.classList.toggle('open', open);
    if (back) back.classList.toggle('open', open);
  }

  // ------------------------------------------------------------------ TOC scrollspy
  function setupScrollspy() {
    var tocLinks = document.querySelectorAll('.toc a[href^="#"]');
    if (!tocLinks.length) return;
    var headings = [];
    tocLinks.forEach(function (a) {
      var id = a.getAttribute('href').slice(1);
      var el = document.getElementById(id);
      if (el) headings.push({ id: id, el: el, link: a });
    });
    if (!headings.length) return;

    var observer = new IntersectionObserver(function (entries) {
      entries.forEach(function (e) {
        if (e.isIntersecting) {
          tocLinks.forEach(function (a) { a.classList.remove('active'); });
          var match = headings.find(function (h) { return h.id === e.target.id; });
          if (match) match.link.classList.add('active');
        }
      });
    }, { rootMargin: '-10% 0px -70% 0px', threshold: 0 });

    headings.forEach(function (h) { observer.observe(h.el); });
  }

  // ------------------------------------------------------------------ Code copy
  function markCopied(btn) {
    var original = btn.textContent;
    btn.classList.add('copied');
    btn.textContent = 'Copied';
    setTimeout(function () {
      btn.classList.remove('copied');
      btn.textContent = original;
    }, 1500);
  }

  // ------------------------------------------------------------------ Global click handler
  document.addEventListener('click', function (e) {
    // Code copy
    var copy = e.target.closest('.code-copy');
    if (copy) {
      var wrap = copy.parentElement;
      var code = wrap && (wrap.querySelector('pre code') || wrap.querySelector('pre'));
      if (code) {
        var text = code.innerText;
        if (navigator.clipboard && navigator.clipboard.writeText) {
          navigator.clipboard.writeText(text).then(function () { markCopied(copy); });
        } else {
          var ta = document.createElement('textarea');
          ta.value = text;
          document.body.appendChild(ta);
          ta.select();
          try { document.execCommand('copy'); markCopied(copy); } catch (_) {}
          document.body.removeChild(ta);
        }
      }
      return;
    }

    // Tabs
    var tabBtn = e.target.closest('.tab-button');
    if (tabBtn) {
      var tabs = tabBtn.closest('.farol-tabs');
      if (tabs) {
        var idx = tabBtn.getAttribute('data-tab');
        tabs.querySelectorAll('.tab-button').forEach(function (b) {
          b.classList.toggle('active', b.getAttribute('data-tab') === idx);
        });
        tabs.querySelectorAll('.tab-panel').forEach(function (p) {
          p.classList.toggle('active', p.getAttribute('data-tab') === idx);
        });
      }
      return;
    }

    // Sidebar toggle / backdrop
    if (e.target.closest('[data-sidebar-toggle]')) {
      sidebarOpen();
      return;
    }
    if (e.target.closest('[data-sidebar-backdrop]')) {
      sidebarOpen(false);
      return;
    }

    // Theme toggle
    if (e.target.closest('[data-theme-toggle]')) {
      toggleTheme();
      return;
    }

    // Close sidebar drawer when clicking a nav link (mobile).
    if (window.matchMedia('(max-width: 960px)').matches && e.target.closest('.sidebar .nav-link')) {
      sidebarOpen(false);
    }
  });

  // ------------------------------------------------------------------ Keyboard
  document.addEventListener('keydown', function (e) {
    if (e.key === 'Escape') {
      var bar = document.querySelector('[data-sidebar]');
      if (bar && bar.classList.contains('open')) sidebarOpen(false);
    }
  });

  // ------------------------------------------------------------------ Init
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', setupScrollspy);
  } else {
    setupScrollspy();
  }
})();
