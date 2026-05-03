(function () {
  // Copy-to-clipboard for code blocks.
  document.addEventListener('click', function (e) {
    var target = e.target;
    if (!target || !target.classList || !target.classList.contains('code-copy')) return;
    var wrap = target.parentElement;
    if (!wrap) return;
    var code = wrap.querySelector('pre code') || wrap.querySelector('pre');
    if (!code) return;
    var text = code.innerText;
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text).then(function () { markCopied(target); });
    } else {
      var ta = document.createElement('textarea');
      ta.value = text;
      document.body.appendChild(ta);
      ta.select();
      try { document.execCommand('copy'); markCopied(target); } catch (_) {}
      document.body.removeChild(ta);
    }
  });

  // Tabs switching.
  document.addEventListener('click', function (e) {
    var btn = e.target;
    if (!btn || !btn.classList || !btn.classList.contains('tab-button')) return;
    var tabs = btn.closest('.farol-tabs');
    if (!tabs) return;
    var idx = btn.getAttribute('data-tab');
    var buttons = tabs.querySelectorAll('.tab-button');
    var panels = tabs.querySelectorAll('.tab-panel');
    buttons.forEach(function (b) { b.classList.toggle('active', b.getAttribute('data-tab') === idx); });
    panels.forEach(function (p) { p.classList.toggle('active', p.getAttribute('data-tab') === idx); });
  });

  function markCopied(btn) {
    var original = btn.textContent;
    btn.classList.add('copied');
    btn.textContent = 'Copied';
    setTimeout(function () {
      btn.classList.remove('copied');
      btn.textContent = original;
    }, 1500);
  }
})();
