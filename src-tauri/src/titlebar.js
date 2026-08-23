// Linux 平台的自定义标题栏注入脚本（后端以 initialization_script 注入，仅在 Linux 生效；
// Windows 用系统原生标题栏，macOS 用系统红绿灯 + 透明标题栏，均无需注入）。
//
// 背景：Linux 已去掉系统标题栏（decorations: false），窗口就绪后会跳转到
// dsh web 页面（http://127.0.0.1:PORT），该页面由 dsh 服务提供、DOM 不受本仓库控制，
// 因此通过本脚本为这类外部页面补一层顶部拖拽条 + 窗口三键（参考 zapmomo 的 WindowControls）。
// 本地 loading 页（tauri origin）自带标题栏控件，这里跳过避免重复。
//
// 融合设计（不遮挡、无分界线）：
// - 标题栏本身透明无背景无边框，直接浮在页面之上；窗口三键颜色随页面主题
//   （body[data-ds-dark-theme]，dsh 页面在 <head> 里按 prefers-color-scheme 设置）自适应。
// - 同时给 html 注入 32px 的 border-box padding，把整个页面上移区域腾给标题栏——
//   dsh 页面为 html/body/#root height:100% 的百分比高度链，border-box 下内容区
//   恰好缩小 32px，页面顶部内容完整下移、不再被遮挡，视觉上标题栏与页面融为一体。
(function () {
  'use strict';

  // 本地 loading 页：tauri://localhost（macOS/Linux WebKit）
  if (location.protocol === 'tauri:') return;

  var BAR_HEIGHT = 32;
  var BAR_ID = 'dsh-work-titlebar';
  var STYLE_ID = 'dsh-work-titlebar-style';

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

  // 标题栏与页面下移的样式。hover 与主题用 CSS 表达（:hover 与
  // body[data-ds-dark-theme] 后代选择器），主题切换即时生效、无需 JS 监听。
  function ensureStyle() {
    if (document.getElementById(STYLE_ID)) return;
    var s = document.createElement('style');
    s.id = STYLE_ID;
    s.textContent =
      'html{padding-top:' + BAR_HEIGHT + 'px !important;box-sizing:border-box !important;}' +
      '#' + BAR_ID + '{position:fixed;top:0;left:0;right:0;height:' + BAR_HEIGHT + 'px;' +
      'z-index:2147483647;display:flex;align-items:stretch;justify-content:flex-end;' +
      'user-select:none;-webkit-user-select:none;}' +
      '#' + BAR_ID + ' button{height:100%;width:40px;display:flex;align-items:center;' +
      'justify-content:center;border:none;background:transparent;cursor:pointer;' +
      'color:#5f6368;transition:background .15s,color .15s;}' +
      '#' + BAR_ID + ' button:hover{background:rgba(0,0,0,0.06);color:#202124;}' +
      '#' + BAR_ID + ' button.close:hover{background:#e81123;color:#fff;}' +
      'body[data-ds-dark-theme] #' + BAR_ID + ' button{color:rgba(255,255,255,0.72);}' +
      'body[data-ds-dark-theme] #' + BAR_ID + ' button:hover{background:rgba(255,255,255,0.12);color:#fff;}';
    document.head.appendChild(s);
  }

  // buildIcon: 向 12x12 的 svg 画布上追加图形元素（全部是静态常量，无外部输入）
  function makeButton(label, cls, buildIcon, onClick) {
    var btn = document.createElement('button');
    btn.type = 'button';
    if (cls) btn.className = cls;
    btn.setAttribute('aria-label', label);
    btn.title = label;
    var svg = svgEl('svg', { width: '12', height: '12', viewBox: '0 0 12 12' });
    buildIcon(svg);
    btn.appendChild(svg);
    btn.addEventListener('click', onClick);
    return btn;
  }

  function line(x1, y1, x2, y2) {
    return svgEl('line', {
      x1: String(x1), y1: String(y1), x2: String(x2), y2: String(y2),
      stroke: 'currentColor', 'stroke-width': '1.2',
    });
  }

  function setup() {
    if (document.getElementById(BAR_ID) || !document.body) return;
    var win = getWin();
    if (!win) return;
    ensureStyle();

    var bar = document.createElement('div');
    bar.id = BAR_ID;

    // 整条可拖拽（按钮是子元素，e.target !== bar，天然排除）；双击切换最大化，贴近原生行为。
    // 依赖 remote-dsh.json 的 IPC 授权（URL 模式需带 :* 端口通配）。
    bar.addEventListener('mousedown', function (e) {
      if (e.target === bar && e.button === 0) win.startDragging();
    });
    bar.addEventListener('dblclick', function (e) {
      if (e.target === bar) win.toggleMaximize();
    });

    bar.appendChild(makeButton('最小化', '', function (svg) {
      svg.appendChild(line(1, 6, 11, 6));
    }, function () { win.minimize(); }));

    bar.appendChild(makeButton('最大化', '', function (svg) {
      svg.appendChild(svgEl('rect', {
        x: '1.5', y: '1.5', width: '9', height: '9',
        fill: 'none', stroke: 'currentColor', 'stroke-width': '1.2',
      }));
    }, function () { win.toggleMaximize(); }));

    bar.appendChild(makeButton('关闭', 'close', function (svg) {
      svg.appendChild(line(1.5, 1.5, 10.5, 10.5));
      svg.appendChild(line(10.5, 1.5, 1.5, 10.5));
    }, function () { win.close(); }));

    document.body.appendChild(bar);
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', setup);
  } else {
    setup();
  }
})();
