Plataforma de crowdfunding descentralizada en Solana. Los usuarios crean campañas de recaudación, reciben donaciones en SOL y reclaman los fondos cuando alcanzan su meta. Si la campaña expira sin llegar a la meta, los donantes pueden pedir reembolso.

¿Cómo funciona?
El admin inicializa la plataforma con una comisión (ej: 2.5%)
Cualquier usuario crea una campaña con título, descripción, meta en SOL y fecha límite
Los donantes envían SOL a la campaña
Si se alcanza la meta, el creador reclama los fondos (se descuenta la comisión)
Si expira sin alcanzar la meta, los donantes piden reembolso
El creador puede cancelar su campaña solo si no tiene donaciones
Tecnologías
Solana
Anchor Framework (Rust)
TypeScript (cliente)
Solana Playground (beta.solpg.io)
Instrucciones disponibles
initialize - Crea la plataforma con la comisión
create_campaign - Crea una campaña nueva
donate - Dona SOL a una campaña
claim_funds - El creador retira los fondos recaudados
cancel_campaign - Cancela campaña sin donaciones
refund - Reembolso si la campaña expiró sin alcanzar meta
Cómo ejecutar en Solana Playground
Abrir beta.solpg.io
Crear proyecto Anchor
Pegar lib.rs en pestaña Program
Click Build y luego Deploy
Obtener SOL de prueba en faucet.solana.com
Pegar client.ts en pestaña Client
Click Run
Seguridad
Solo el creador puede reclamar o cancelar su campaña
No se aceptan donaciones después de la fecha límite
No se puede reclamar dos veces
Reembolso solo si la meta no se cumplió y la campaña expiró
Cancelación solo si la campaña tiene cero donaciones
Comisión
Se cobra un porcentaje al creador cuando reclama los fondos. El porcentaje se define al inicializar la plataforma en basis points (250 = 2.5%). La comisión se envía al wallet del admin.
