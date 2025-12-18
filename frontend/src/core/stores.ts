import { writable } from 'svelte/store';

function createThemeStore() {
  const saved = localStorage.getItem('theme') || 'light';
  const { subscribe, set } = writable(saved);

  return {
    subscribe,
    set: (value: string) => {
      localStorage.setItem('theme', value);
      document.documentElement.setAttribute('data-theme', value);
      set(value);
    }
  };
}

export const theme = createThemeStore();