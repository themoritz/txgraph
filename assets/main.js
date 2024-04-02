function isDarkMode() {
  return window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches;
}

function addRouteListener(callback) {
  window.addEventListener('popstate', (event) => {
    event.preventDefault();
    callback(window.location.pathname);
  });

  callback(window.location.pathname);
}

function pushHistoryState(url) {
  window.history.pushState({}, '', url);
}

function getRandom() {
  return Math.random()
}
