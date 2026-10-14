# Guía de uso

Esta guía explica cómo leer cada parte de IdleZero con ejemplos. Los tiempos corresponden al mapa de referencia (Arabia) y son aproximados.

## 1. Leer un plan

Elige la plantilla **Edad de los Castillos lo antes posible** y pulsa **Calcular**.

![Mapa de economía](img/mapa.png)

El mapa se lee de abajo (inicio, 0:00) hacia arriba (meta):

- **Centro:** el centro urbano. Cada círculo es un aldeano que nace. El color del carril compara el número de aldeanos con un rango habitual para ese objetivo.
- **Izquierda:** los edificios, con su hora de inicio, duración y constructores.
- **Derecha:** una columna por recurso y una línea por aldeano.
- **Extremo derecho:** las reservas de cada recurso a lo largo del tiempo.
- **Bandas horizontales:** el clic de cada edad y el momento en que se alcanza.

El panel lateral resume el tiempo total, si el centro urbano estuvo parado y cómo queda la economía al terminar.

## 2. Seguir a un aldeano

En la fila **Aldeanos**, haz clic en uno. Su recorrido se resalta y el panel muestra qué hizo en cada momento.

![Un aldeano y sus alternativas](img/foco.png)

La sección **Si este aldeano hiciera otra cosa** compara su decisión con las demás. Por ejemplo, «+37 s · CU parado 12 s» significa que el objetivo llegaría 37 s más tarde y el centro urbano se quedaría 12 s sin comida para crear aldeanos. Las flechas `←` y `→` pasan de un aldeano a otro.

## 3. Un objetivo con unidades

Con **+ Unidad** se abre la lista de unidades por tipo y edad. Al elegir una, su ficha muestra costo, estadísticas, a qué unidades es vulnerable y las mejoras relacionadas, que pueden añadirse al objetivo.

![Ficha de unidad](img/carta.png)

En el **Tablero**, la gráfica del objetivo marca cuándo hubo recursos para pagar cada unidad o edificio y cuándo estuvo listo.

![Gráfica del objetivo](img/objetivo.png)

## 4. Civilizaciones

El botón del escudo abre la lista de civilizaciones. Cada una muestra qué bonos se aplican en la simulación y cuáles no afectan a la economía. Al aceptar, el mismo objetivo se recalcula y se compara con el escenario sin civilización.

![Civilizaciones](img/civilizaciones.png)

## 5. Batalla

La pestaña **Batalla** enfrenta dos ejércitos con su civilización, edad y mejoras, y muestra el resultado y la vida de cada ejército en el tiempo.

![Batalla](img/batalla.png)

En **Duelo de estrategias** se eligen dos plantillas. IdleZero calcula ambas economías y compara, minuto a minuto, qué ejército tendría ventaja si se enfrentaran en ese momento.

![Duelo de estrategias](img/duelo.png)

## Preguntas frecuentes

**¿Por qué el tiempo real es distinto?** Las tasas y distancias son aproximadas y la partida real tiene desplazamientos y reacciones que el modelo no incluye. Lo más útil es comparar planes entre sí.

**¿Se puede detener un cálculo?** Sí, mientras calcula el botón permite cancelar.
