// 非 macOS 平台的自定义标题栏注入脚本（后端以 initialization_script 注入，仅在 Windows/Linux 生效）。
//
// 背景：Windows/Linux 已去掉系统标题栏（decorations: false），窗口就绪后会跳转到
// dsh web 页面（http://127.0.0.1:PORT），该页面由 dsh 服务提供、DOM 不受本仓库控制，
// 因此通过本脚本为这类外部页面补一层顶部拖拽条 + 窗口三键（参考 zapmomo 的 WindowControls）。
// 本地 loading 页（tauri origin）自带标题栏控件，这里跳过避免重复。
(function () {
  'use strict';

  // 本地 loading 页：tauri://localhost（macOS/Linux WebKit）或 http://tauri.localhost（Windows）
  if (location.protocol === 'tauri:' || location.hostname === 'tauri.localhost') return;
  // 双保险：macOS 由系统红绿灯 + 透明标题栏承担，后端本就不会在 macOS 注入本脚本
  if (navigator.userAgent.includes('Macintosh')) return;

  var SVG_NS = 'http://www.w3.org/2000/svg';

  function getWin() {
    return window.__TAURI__ && window.__TAURI__.window
      ? window.__TAURI__.window.getCurrentWindow()
      : null;
  }

  function svgEl(tag, attrs) {
    var el = document.createElementNS(SVG_NS, tag);
    for (var k in attrs) el.setAttribute(k, attrs[k]);
    return el;
  }

  var BTN_BASE =
    'height:100%;width:40px;display:flex;align-items:center;justify-content:center;' +
    'border:none;background:transparent;cursor:pointer;color:#5f6368;' +
    'transition:background .15s,color .15s;';

  // buildIcon: 向 12x12 的 svg 画布上追加图形元素（全部是静态常量，无外部输入）
  function makeButton(label, buildIcon, hoverCss, onClick) {
    var btn = document.createElement('button');
    btn.type = 'button';
    btn.setAttribute('aria-label', label);
    btn.title = label;
    btn.style.cssText = BTN_BASE;
    var svg = svgEl('svg', { width: '12', height: '12', viewBox: '0 0 12 12' });
    buildIcon(svg);
    btn.appendChild(svg);
    btn.addEventListener('mouseenter', function () {
      btn.style.cssText = BTN_BASE + hoverCss;
    });
    btn.addEventListener('mouseleave', function () {
      btn.style.cssText = BTN_BASE;
    });
    btn.addEventListener('click', onClick);
    return btn;
  }

  function line(x1, y1, x2, y2) {
    return svgEl('line', {
      x1: String(x1), y1: String(y1), x2: String(x2), y2: String(y2),
      stroke: 'currentColor', 'stroke-width': '1.2',
    });
  }

  var HOVER_NORMAL = 'background:rgba(0,0,0,0.06);color:#202124;';
  var HOVER_CLOSE = 'background:#e81123;color:#fff;';

  function setup() {
    if (document.getElementById('dsh-work-titlebar') || !document.body) return;
    var win = getWin();
    if (!win) return;

    var bar = document.createElement('div');
    bar.id = 'dsh-work-titlebar';
    bar.style.cssText =
      'position:fixed;top:0;left:0;right:0;height:32px;z-index:2147483647;' +
      'display:flex;align-items:stretch;justify-content:flex-end;' +
      'background:rgba(255,255,255,0.85);backdrop-filter:blur(8px);-webkit-backdrop-filter:blur(8px);' +
      'border-bottom:1px solid rgba(0,0,0,0.08);user-select:none;-webkit-user-select:none;';

    // 整条可拖拽（按钮是子元素，e.target !== bar，天然排除）；双击切换最大化，贴近原生行为
    bar.addEventListener('mousedown', function (e) {
      if (e.target === bar && e.button === 0) win.startDragging();
    });
    bar.addEventListener('dblclick', function (e) {
      if (e.target === bar) win.toggleMaximize();
    });

    bar.appendChild(makeButton('最小化', function (svg) {
      svg.appendChild(line(1, 6, 11, 6));
    }, HOVER_NORMAL, function () { win.minimize(); }));

    bar.appendChild(makeButton('最大化', function (svg) {
      svg.appendChild(svgEl('rect', {
        x: '1.5', y: '1.5', width: '9', height: '9',
        fill: 'none', stroke: 'currentColor', 'stroke-width': '1.2',
      }));
    }, HOVER_NORMAL, function () { win.toggleMaximize(); }));

    bar.appendChild(makeButton('关闭', function (svg) {
      svg.appendChild(line(1.5, 1.5, 10.5, 10.5));
      svg.appendChild(line(10.5, 1.5, 1.5, 10.5));
    }, HOVER_CLOSE, function () { win.close(); }));

    document.body.appendChild(bar);
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', setup);
  } else {
    setup();
  }
})();
