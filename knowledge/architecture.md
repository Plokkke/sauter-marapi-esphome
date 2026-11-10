# Architecture

## Rôles

| Niveau | Projet | Rôle |
|---|---|---|
| Sèche-serviettes | Sauter Marapi tactile 1750 W (750 W de fluide + 1000 W de soufflerie) | sa carte de commande (écran, touches) choisit consigne, mode et Boost ; sa carte de puissance chauffe d'après sa propre sonde |
| Pont | **ce projet** | un ESP32-C3 sur le bus Linap interne : lit ce que fait l'utilisateur, impose ce que décide Home Assistant |
| Maison | automatisation HA (autre projet) | plannings, présence, délestage |

Ce projet ne contient aucune automatisation HA.

## Solution retenue : un ESP au milieu du bus Linap

**Retenu** (11/2026) : un **MITM** sur la nappe entre la carte de commande et la carte de puissance (protocole : `linap.md`).

- DATA est coupé, GND et VCC traversent. L'ESP joue la carte de puissance face à la carte de commande, et la carte de commande face à la carte de puissance : les deux moitiés ont été validées séparément (`discovery/`).
- **L'écran et les touches d'origine restent** : pas d'interface à refaire.
- **Par défaut, l'ESP relaie** : le radiateur se comporte exactement comme sans lui.
- **Home Assistant peut imposer** une consigne, le Boost ou l'arrêt : l'ESP l'envoie à la carte de puissance à la place de l'état de la carte de commande, jusqu'au prochain changement fait aux touches, qui reprend la main.
- **Limite acceptée : l'écran n'est pas synchronisé.** La carte de commande n'affiche que ce qu'on règle sur ses touches ; un Boost lancé par HA ne s'y voit pas. HA, lui, sait tout : réglages faits aux touches, chauffe en cours.

Écartés :

- **Fil pilote + touche Boost simulée** (`fil-pilote.md`, carte `hardware/pilot-wire/`) : fonctionnait sans rien connaître du radiateur, mais sans retour d'état et avec un faux doigt sur la touche Boost.
- **Contrôleur maison à la place de la carte de commande** : écran toujours juste, mais toute l'interface (touches, affichage, programme) à refaire. Reste le repli si le MITM échoue.

## Entités prévues

| Entité | Type | Rôle |
|---|---|---|
| Consigne, Boost, arrêt lus sur la carte de commande | capteurs | ce que l'utilisateur a réglé aux touches |
| Chauffe en cours | capteur binaire | `C7` de la carte de puissance |
| Mode imposé | `select` | relais (défaut), consigne imposée, Boost, arrêt |
| Consigne imposée | `number` | consigne envoyée à la carte de puissance en mode imposé |

La carte de commande n'envoie que sa consigne effective : HA voit la consigne Éco quand elle est en Éco, sans le mot « Éco ».

## Sécurité

- **Repli matériel** : si l'ESP n'est pas alimenté, plante ou démarre, un relais à contact repos relie directement les deux moitiés de DATA, et le radiateur fonctionne comme d'origine. Sans lui, un ESP en panne mettrait le radiateur en « Err 10 ».
- **Repli logiciel** : sans HA pendant 30 min, l'ESP revient au relais.
- **Isolement** : le bus est au potentiel du secteur ; tout ce qui est touchable (USB, boîtier) en est isolé (HLK-PM01, optocoupleurs).

## Alimentation

- Le module est alimenté par l'arrivée murale du sèche-serviettes (phase + neutre, en aval du différentiel 30 mA de la salle de bains), via un HLK-PM01 protégé par un fusible.
- La nappe se raccorde au module par un câble prévu pour le secteur : un côté vers la carte de commande, un côté vers la carte de puissance.
