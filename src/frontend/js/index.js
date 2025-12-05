let currentPath = '';

// Theme management
const themeToggle = document.getElementById('themeToggle');
const html = document.documentElement;

// Load saved theme
const savedTheme = localStorage.getItem('theme') || 'dark';
html.setAttribute('data-theme', savedTheme);
updateThemeButton(savedTheme);

themeToggle.addEventListener('click', () => {
  const currentTheme = html.getAttribute('data-theme');
  const newTheme = currentTheme === 'dark' ? 'light' : 'dark';
  html.setAttribute('data-theme', newTheme);
  localStorage.setItem('theme', newTheme);
  updateThemeButton(newTheme);
});

function updateThemeButton(theme) {
  if (theme === 'dark') {
    themeToggle.textContent = '☀️ Light';
  } else {
    themeToggle.textContent = '🌙 Dark';
  }
}

async function loadDirectory(path, pushState = true) {
  const fileList = document.getElementById('fileList');
  fileList.innerHTML = '<li class="loading">Loading...</li>';

  try {
    const url = path ? `/api/list/${path}` : '/api/list';
    const response = await fetch(url);

    if (!response.ok) {
      throw new Error('Failed to load directory');
    }

    const files = await response.json();
    currentPath = path;
    updateBreadcrumb(path);
    renderFiles(files);

    // Update URL without reloading page
    if (pushState) {
      const newUrl = path ? `/${path}` : '/';
      window.history.pushState({ path }, '', newUrl);
    }
  } catch (error) {
    fileList.innerHTML = `<li class="error">Error loading directory: ${error.message}</li>`;
  }
}

function updateBreadcrumb(path) {
  const breadcrumb = document.getElementById('breadcrumb');
  // Normalize path separators to forward slashes
  const normalizedPath = path.replace(/\\/g, '/');
  const parts = normalizedPath ? normalizedPath.split('/') : [];

  let html = '<a data-path="">storage</a>';
  let accumulated = '';

  parts.forEach((part) => {
    accumulated += (accumulated ? '/' : '') + part;
    html += ` / <a data-path="${accumulated}">${part}</a>`;
  });

  breadcrumb.innerHTML = html;

  // Add click handlers
  breadcrumb.querySelectorAll('a').forEach(link => {
    link.addEventListener('click', (e) => {
      e.preventDefault();
      loadDirectory(link.dataset.path);
    });
  });
}

function renderFiles(files) {
  const fileList = document.getElementById('fileList');

  if (files.length === 0) {
    fileList.innerHTML = '<li class="empty-state">This folder is empty</li>';
    return;
  }

  fileList.innerHTML = files.map(file => {
    const icon = file.is_dir ? '📁' : '📄';
    const className = file.is_dir ? 'directory' : 'file';
    const size = file.is_dir ? '-' : formatSize(file.size);
    const date = formatDate(file.modified);

    return `
                  <li class="file-item ${className}" data-path="${file.path}" data-is-dir="${file.is_dir}">
                      <span class="file-icon">${icon}</span>
                      <span class="file-name">${file.name}</span>
                      <span class="file-size">${size}</span>
                      <span class="file-date">${date}</span>
                  </li>
              `;
  }).join('');

  // Add click handlers
  fileList.querySelectorAll('.file-item').forEach(item => {
    item.addEventListener('click', () => {
      const path = item.dataset.path;
      const isDir = item.dataset.isDir === 'true';

      if (isDir) {
        loadDirectory(path);
      } else {
        window.location.href = `/files/${path}`;
      }
    });
  });
}

function formatSize(bytes) {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i];
}

function formatDate(timestamp) {
  const date = new Date(timestamp * 1000);
  const now = new Date();
  const diffMs = now - date;
  const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

  if (diffDays === 0) {
    return 'Today ' + date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });
  } else if (diffDays === 1) {
    return 'Yesterday';
  } else if (diffDays < 7) {
    return diffDays + ' days ago';
  } else {
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
  }
}

// Handle browser back/forward buttons
window.addEventListener('popstate', (event) => {
  const path = event.state?.path || '';
  loadDirectory(path, false);
});

// Load directory from URL path on page load
const initialPath = window.location.pathname.replace(/^\/+/, '');
// Only load if it's not an API or files route
if (!initialPath.startsWith('api/') && !initialPath.startsWith('files/')) {
  loadDirectory(initialPath, false);
}