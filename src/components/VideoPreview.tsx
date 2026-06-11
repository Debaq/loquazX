function VideoPreview() {
  return (
    <div className="preview">
      <div className="preview__placeholder">
        <div className="preview__icon">▶</div>
        <div className="preview__hint">Sin video cargado</div>
        <div className="preview__sub">Arrastra un archivo o usa Abrir</div>
      </div>
    </div>
  );
}

export default VideoPreview;
