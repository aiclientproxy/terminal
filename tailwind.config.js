/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // 终端主题颜色
        terminal: {
          bg: '#1e1e1e',
          fg: '#d4d4d4',
          cursor: '#aeafad',
          selection: '#264f78',
        },
      },
    },
  },
  plugins: [],
}
