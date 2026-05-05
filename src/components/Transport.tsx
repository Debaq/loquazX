function Transport() {
  return (
    <div className="transport">
      <div className="transport__controls">
        <button type="button" disabled>⏮</button>
        <button type="button" disabled>▶</button>
        <button type="button" disabled>⏭</button>
      </div>
      <div className="transport__time">00:00.0 / 00:00.0</div>
      <div className="transport__bar">
        <div className="transport__progress" style={{ width: "0%" }} />
      </div>
    </div>
  );
}

export default Transport;
