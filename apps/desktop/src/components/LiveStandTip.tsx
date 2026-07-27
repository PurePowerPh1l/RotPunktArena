import { useEffect, useState } from "react";
import { formatLiveStandTip, pickLiveStandTip } from "../liveStandTips";

/** Rotating tip / Easter egg next to the brand in the Arena top bar. */
export function LiveStandTip() {
  const [tip, setTip] = useState(() => pickLiveStandTip());

  useEffect(() => {
    const id = window.setInterval(() => {
      setTip((prev) => pickLiveStandTip(prev));
    }, 18_000);
    return () => window.clearInterval(id);
  }, []);

  return (
    <p className="hint top-tip" key={tip} title={tip}>
      {formatLiveStandTip(tip)}
    </p>
  );
}
