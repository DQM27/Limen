// El tiempo real del panel: la página se conecta directo a la base (WebSocket) y la base le avisa
// al instante de lo que cambia. Para entrar necesita un token, y ese token lo da este Worker a
// quien tenga la clave del panel. El token es de sólo lectura y vale una hora: lo que puede ver lo
// deciden los permisos de la base para el acceso `panel` (`docs/nube.md`, sección 14), no la página.

import { VIGENCIA_TOKEN_S, firmarJwtPanel } from './token.js';

export async function enVivo(env, ahora = Date.now()) {
  if (!env.PANEL_VIVO_SECRETO) {
    // Todavía no se configuró el secreto en el Worker: la página sigue funcionando sin tiempo real.
    return { estado: 503, cuerpo: { error: 'en_vivo_sin_configurar' } };
  }
  return {
    estado: 200,
    cuerpo: {
      token: await firmarJwtPanel(env.PANEL_VIVO_SECRETO, ahora),
      url: env.SURREAL_URL.replace(/^http/, 'ws'),
      vence: ahora + VIGENCIA_TOKEN_S * 1000,
    },
  };
}
