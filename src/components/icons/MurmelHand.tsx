import { useId } from "react";

/**
 * Murmel-Bildmarke: eine Glaskugel, über die ein dunkles Band diagonal von
 * oben rechts nach unten links läuft.
 *
 * Zum Volumen, das ohne einen zweiten Farbton auskommt:
 *
 * 1. Das Band ist eine FLÄCHE, kein Strich — in der Mitte breit, zu den
 *    Rändern spitz zulaufend. Genau so verkürzt sich ein Band perspektivisch,
 *    das sich um eine Kugel legt. Ein Strich mit fester Breite sähe aufgeklebt
 *    aus; erst diese Taillierung erzeugt Rundung.
 * 2. Ein radialer Verlauf mit Licht von oben links, dazu ein Glanzlicht und
 *    eine leicht abgedunkelte Kante. Alle Töne stammen aus der warmen
 *    Papierfamilie (theme.css) — monochrom heißt eine Farbfamilie, nicht
 *    zwei Werte.
 *
 * Die Verlaufs-IDs werden pro Instanz erzeugt (`useId`); mit festen IDs würden
 * sich mehrere gleichzeitig gerenderte Murmeln im Dokument gegenseitig
 * überschreiben.
 */
const MurmelHand = ({
  width,
  height,
}: {
  width?: number | string;
  height?: number | string;
}) => {
  const uid = useId();
  const ball = `murmel-ball-${uid}`;
  const sphere = `murmel-sphere-${uid}`;
  const spec = `murmel-spec-${uid}`;
  const edge = `murmel-edge-${uid}`;

  return (
    <svg
      width={width || 128}
      height={height || 128}
      viewBox="0 0 128 128"
      xmlns="http://www.w3.org/2000/svg"
    >
      <defs>
        <clipPath id={ball}>
          <circle cx="64" cy="64" r="52" />
        </clipPath>

        {/* Kugelvolumen — Licht von oben links */}
        <radialGradient id={sphere} cx="34%" cy="28%" r="78%">
          <stop offset="0%" stopColor="#ffffff" />
          <stop offset="42%" stopColor="var(--color-logo-fill)" />
          <stop offset="78%" stopColor="var(--color-logo-mid)" />
          <stop offset="100%" stopColor="var(--color-logo-shade)" />
        </radialGradient>

        {/* Glanzlicht */}
        <radialGradient id={spec} cx="50%" cy="50%" r="50%">
          <stop offset="0%" stopColor="#ffffff" stopOpacity="0.95" />
          <stop offset="100%" stopColor="#ffffff" stopOpacity="0" />
        </radialGradient>

        {/* Randabdunklung — lässt die Kugel zur Kante hin wegdrehen */}
        <radialGradient id={edge} cx="50%" cy="50%" r="50%">
          <stop
            offset="82%"
            stopColor="var(--color-logo-stroke)"
            stopOpacity="0"
          />
          <stop
            offset="100%"
            stopColor="var(--color-logo-stroke)"
            stopOpacity="0.3"
          />
        </radialGradient>
      </defs>

      <circle cx="64" cy="64" r="52" fill={`url(#${sphere})`} />

      {/* Das Band: zwei Bögen, die an den Kugelrändern zusammenlaufen */}
      <path
        d="M100.8 27.2 C 58 34, 62 87, 27.2 100.8 C 70 94, 66 41, 100.8 27.2 Z"
        fill="var(--color-logo-stroke)"
        clipPath={`url(#${ball})`}
      />

      <ellipse
        cx="44"
        cy="40"
        rx="19"
        ry="13"
        fill={`url(#${spec})`}
        transform="rotate(-30 44 40)"
      />

      <circle cx="64" cy="64" r="52" fill={`url(#${edge})`} />

      {/* Feine Kontur — gibt der Kugel auf hellem Papier eine klare Kante,
          ohne das Volumen flachzudrücken. */}
      <circle
        cx="64"
        cy="64"
        r="52"
        fill="none"
        stroke="var(--color-logo-stroke)"
        strokeWidth="3"
      />
    </svg>
  );
};

export default MurmelHand;
