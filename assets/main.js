function isDarkMode() {
  return window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches;
}

function addRouteListener(callback) {
  window.addEventListener('popstate', (event) => {
    event.preventDefault();
    callback(window.location.href);
  });

  callback(window.location.href);
}

function pushHistoryState(url) {
  window.history.pushState({}, '', url);
}
