/* eslint-disable i18next/no-literal-string -- Wortmarke: der Produktname wird nicht übersetzt. */

/**
 * Murmel-Wortmarke, gesetzt in Fraunces.
 *
 * Fraunces ist eine variable Serifenschrift mit weichen, leicht organischen
 * Rundungen (SOFT-Achse) — das trifft eine Murmel besser als der harte
 * Strichkontrast einer Didone. Sie ist über `@fontsource-variable/fraunces`
 * lokal gebündelt (Import in `main.tsx`), lädt also nichts nach.
 *
 * `opsz` steht bewusst am oberen Ende: Fraunces zeichnet bei großen optischen
 * Größen feinere Ansätze und mehr Kontrast — genau das, was eine Wortmarke
 * braucht und Fließtext nicht verträgt.
 *
 * Bewusst als SVG-Text gesetzt (statt als Pfade), damit der Schriftzug ohne
 * fremde Markenassets auskommt und über `currentColor` dem Theme folgt.
 * Die viewBox bleibt beim Seitenverhältnis der bisherigen Wortmarke, damit
 * aufrufende Layouts (`width={120}` / `width={200}`) unverändert passen.
 */
const MurmelTextLogo = ({
  width,
  height,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => {
  return (
    <svg
      width={width}
      height={height}
      className={className}
      viewBox="0 0 930 328"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      role="img"
      aria-label="Murmel"
      /* Ohne dies rendert Windows den großen SVG-Text mit Subpixel-Glättung
         (ClearType) und die Buchstaben bekommen farbige Ränder — in einer
         monochromen Wortmarke besonders auffällig. */
      style={{ textRendering: "geometricPrecision" }}
      shapeRendering="geometricPrecision"
    >
      <text
        x="0"
        y="238"
        fill="currentColor"
        fontFamily="'Fraunces Variable', Fraunces, Georgia, 'Times New Roman', serif"
        fontSize="228"
        fontWeight="600"
        letterSpacing="-4"
        style={{ fontVariationSettings: "'opsz' 144, 'SOFT' 30, 'WONK' 0" }}
      >
        Murmel
      </text>
    </svg>
  );
};

export default MurmelTextLogo;
