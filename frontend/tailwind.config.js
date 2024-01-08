const plugin = require('tailwindcss/plugin');

/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.rs'],
  theme: {
    extend: {},
  },
  plugins: [
    plugin(({ addBase }) =>
      addBase({
        html: {
          fontSize: '16px',
        },
      })
    ),
    require('@tailwindcss/typography'),
    require('@tailwindcss/forms'),
  ],
};
