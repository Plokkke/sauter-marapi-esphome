# Backlog

## Fait

- [x] Bus Linap décodé : format, dialogue, consigne, Boost, arrêt (`linap.md`).
- [x] La carte de puissance obéit à un ESP qui joue la carte de commande (consigne, Boost, arrêt).
- [x] La carte de commande répond à un ESP qui joue la carte de puissance : tout ce qui est fait aux touches est lisible.
- [x] La carte de commande ne se laisse pas piloter par le bus (adresses et écritures essayées) : l'écran ne suivra pas HA.
- [x] Capacité du 5 V de la nappe : ≈ 160 mA en tout, lâche vers 180 mA. Un D1 mini y tient avec le boîtier de commande (démarrages et Boost validés) : plus de 230 V à raccorder.

## Carte MITM

- [ ] Redessiner `hardware/linap/` pour le MITM :
  - sur un D1 mini alimenté par la nappe, avec un condensateur de 470 à 1000 µF ;
  - deux voies, côté commande et côté puissance, chacune avec le montage du banc (pont diviseur en lecture, NPN en émission) ;
  - le rappel de DATA à l'état haut côté carte de puissance (≈ 5 kΩ vers VCC), que la carte de commande ne fait plus ;
  - deux connecteurs pour la nappe coupée (GND, VCC, DATA de chaque côté).
- [ ] Mesurer la place libre dans le boîtier de commande et choisir où loger la carte et comment couper la nappe (rallonge à connecteurs d'origine si possible).
- [ ] Mesurer la marge de réponse de la carte de puissance (étape 3, réglage « Reply gap » à 2, 5, 10, 20 bits, Boost à chaque palier), pour savoir si l'ESP8266 du D1 mini répond assez vite (une seule UART complète : UART0 côté puissance, UART1 et réception logicielle côté commande). Un ESP32 consommerait trop pour la nappe.
- [ ] Firmware : réponse dans le bit qui suit l'appel (réception sur interruption ou UART matérielle), mode transparent par défaut, état imposé par HA, retour au mode transparent sur appui d'une touche ou sans HA pendant 30 min.
- [ ] Prototype au banc avec deux D1 mini ou un ESP32 à deux UART avant la carte : vérifier le mode transparent et l'état imposé, cartes branchées.
- [ ] Boîtier (`enclosure/`), séparation secteur / basse tension.

## À comprendre

- [ ] Capture longue avec une consigne proche de la température de la pièce : trouver la mesure de température dans `75`, `61` ou `73`, pour l'exposer à HA.
- [ ] Ce que fait la carte de puissance après un long silence de la carte de commande (au-delà de 30 s).
- [ ] Sens de la réponse `70` (`83 80 89`, constante) et de `76`.

## Côté Home Assistant

- [ ] Entités du MITM (`architecture.md`), puis thermostat HA dans le projet d'automatisation.
- [ ] Mesure de puissance éventuelle : Shelly PM Mini Gen3 dans la boîte de raccordement (≈ 1750 W = Boost, ≈ 750 W = fluide, 0 = attente), utile si `C7` ne suffit pas.

## Référence : carte de commande

Photographiée le 25/10/2026 ([dans son boîtier](photos/command-board-in-box.jpg), [touches et écran](photos/command-board-keys.jpg)) : étiquette `HM011 TD1N01 RAD`, circuit imprimé `74 60 5004 IndB`, microcontrôleur **STM32F091CCT6** (contrôleur tactile intégré), écran LCD Varitronix, touches capacitives (ressorts dorés vers la vitre), 5 pastilles alignées (probablement SWD, lecture du firmware sûrement verrouillée). La garantie (2 ans) est perdue dès qu'on ouvre le boîtier.
