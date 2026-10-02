# Native dependency artifact. Receiving products embed these verified files;
# this image neither starts a server nor owns the host's orchestration.
FROM scratch
ARG MUSTEROFFICE_SOURCE_REVISION
ARG MUSTEROFFICE_RELEASE_VERSION
LABEL org.opencontainers.image.title="MusterOffice native component" \
      org.opencontainers.image.revision=$MUSTEROFFICE_SOURCE_REVISION \
      org.opencontainers.image.version=$MUSTEROFFICE_RELEASE_VERSION
COPY component/ /opt/musteroffice/
