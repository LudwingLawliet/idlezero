# Guía de IdleZero

Esta guía recorre IdleZero con cinco casos reales. Cada uno tarda un par de minutos. Las capturas están en `docs/img/`.

## 1. Tu primer Fast Castle

1. En **Objetivo**, abre *Plantillas…* y elige **Edad de los Castillos lo antes posible**.
2. Pulsa **Calcular**. En unos segundos aparece el mapa.
3. Lee el mapa de abajo hacia arriba:
   - en el centro, el **CU** crea un aldeano cada 25 s (cada círculo es un nacimiento);
   - cada aldeano sale hacia su recurso a la derecha (ovejas, jabalí, bayas, madera, oro) o hacia una construcción a la izquierda;
   - las **bandas** horizontales marcan cuándo haces clic en cada edad y cuándo llegas.
4. En el panel derecho, el **Resumen** da el tiempo total (14:25 en Arabia neutra), los aldeanos y si el CU estuvo parado (debería decir 0 s).
5. En **Al cumplir el objetivo** ves cómo queda tu economía al llegar a Castillos: cuántos aldeanos hay en cada recurso y cuánto ingresas por minuto.

![Mapa de economía](img/mapa.png)

Consejo: pasa a la pestaña **Lista** (`3`) y pulsa *Copiar como texto* para tener la orden en papel o en una segunda pantalla.

## 2. Entender un aldeano

1. En la fila **Aldeanos** de abajo, haz clic en **a6**. Las demás ramas se apagan y la vista se centra en su camino.
2. El panel muestra **qué hace y cuándo** (jabalí de 1:15 a 7:15, luego bayas) y lo que juntó.
3. En **Si este aldeano hiciera otra cosa** ves el costo de cambiar su decisión: «madera +37 s · CU parado 12 s» significa que llegarías 37 s más tarde y el CU se quedaría sin comida 12 s.
4. Usa `←` y `→` para pasar por todos los aldeanos en orden.

![Un aldeano y sus alternativas](img/foco.png)

Si un tramo aparece en **rojo**, ese aldeano estuvo sin tarea: el panel dice por qué (por ejemplo «sin madera para su granja») y cómo evitarlo («guarda 60 de madera por granja…»). Con los planes de IdleZero no debería pasar; sirve sobre todo al comparar caminos.

## 3. Planificar 10 caballeros

1. Quita la edad del objetivo (la `×` del chip) y pulsa **+ Unidad**.
2. En la pestaña **Caballería**, elige **Caballero**. La carta muestra su costo, sus estadísticas y que es débil contra lanceros y camellos (las unidades con bonos de ataque contra montados).
3. En **Mejoras relacionadas** deja *Recomendadas* (Forja, Barda de escamas, Línea de sangre) o elige *Ninguna*. Pon la cantidad en 10 y pulsa **Añadir al objetivo**.
4. Calcula y abre el **Tablero** (`2`). La gráfica **¿Cuándo tienes cada cosa del objetivo?** pone una insignia por caballero en el segundo exacto en que pudiste pagarlo y una barra hasta que salió del establo.

![Carta de unidad](img/carta.png)
![Gráfica del objetivo](img/objetivo.png)

## 4. Probar otra civilización

1. Pulsa el escudo (o `C`) y escribe «fran».
2. A la derecha ves en verde lo que IdleZero simula de los francos (recolectores +15 %, tecnologías de molino gratis) y en gris lo que no afecta a la economía.
3. **Aceptar** recalcula el mismo objetivo. El resumen compara con el escenario neutro («−25 s»).

![Civilizaciones](img/civilizaciones.png)

## 5. ¿Quién gana y cuándo?

1. Abre **Batalla** (`5`).
2. En cada bando elige civilización, edad y ejército (con **+ Unidad**; las mejoras que marques en la carta se aplican al combate). Pulsa **¡A pelear!**.
3. En **Duelo de estrategias** elige dos plantillas, por ejemplo *10 caballeros* para A y *Arqueros en Feudal* para B, y pulsa **Comparar estrategias**. Las barras muestran quién ganaría si los ejércitos se enfrentaran en cada minuto: los arqueros mandan antes (minutos 13–15) y los caballeros después (desde el 16).

![Batalla](img/batalla.png)
![Duelo de estrategias](img/duelo.png)

Recuerda los límites: sin terreno, micro ni formaciones. Úsalo para comparar ideas, no para predecir una partida.

## Preguntas frecuentes

**¿Por qué mi tiempo real es distinto?** Las tasas de recolección y las distancias son aproximadas, y en el juego hay caminatas, empujar ciervos y reacción humana. Lo útil es la **diferencia** entre dos planes, más que el segundo exacto.

**¿Por qué el CU sale amarillo o rojo?** Llevas más aldeanos de los habituales para ese objetivo (por ejemplo más de 24 para un Feudal rápido). No es un error: a veces conviene, pero tarda más.

**¿Puedo cancelar un cálculo?** Sí: mientras calcula, el botón dice *Calculando… · Cancelar*. Un objetivo de Imperial puede tardar 20–30 s en un equipo normal.

**¿Dónde guardo mis capturas?** En `docs/img/`, enlazadas desde el README o esta guía.
