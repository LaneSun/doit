import adapter from '@sveltejs/adapter-static';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  kit: {
    // 纯 SPA:生成 index.html 作为回退,由客户端路由接管
    adapter: adapter({ fallback: 'index.html' })
  }
};

export default config;
