#!/bin/bash
# settings.yaml 파일에서 IP를 추출하여 YAML 파일의 $(HOST_IP) 패턴을 대체하는 스크립트

# 입력 및 출력 파일 경로
SETTINGS_YAML="/etc/containers/systemd/piccolo/settings.yaml"
SERVER_YAML="/etc/containers/systemd/piccolo/piccolo-server.yaml"
PLAYER_YAML="/etc/containers/systemd/piccolo/piccolo-player.yaml"

# settings.yaml에서 IP 주소 추출
HOST_IP=$(grep -A 3 "host:" $SETTINGS_YAML | grep "ip:" | sed -e "s/^[ ]*ip:[ ]*//" -e "s/[ ]*$//")
echo "Extracted HOST_IP: $HOST_IP"

# IP 주소가 비어있는지 확인
if [ -z "$HOST_IP" ]; then
  echo "Failed to extract host IP from settings.yaml"
  exit 1
fi

# 문자 그대로의 $(HOST_IP)를 실제 IP 주소로 대체
sed -i "s/\\\$(HOST_IP)/${HOST_IP}/g" $SERVER_YAML

echo "Successfully replaced \$(HOST_IP) with $HOST_IP in $SERVER_YAML"
exit 0

#
# Get version info and update container image version and nodeagent in release note
#
VERSION_TXT="/etc/containers/systemd/piccolo/version.txt"
VERSION=$(cat $VERSION_TXT)
echo "Version is: $VERSION"

sed -i "s/\\\$(VERSION)/${VERSION}/g" $SERVER_YAML
sed -i "s/\\\$(VERSION)/${VERSION}/g" $PLAYER_YAML

echo "Successfully replaced \$(VERSION) with $VERSION in $SERVER_YAML"
echo "Successfully replaced \$(VERSION) with $VERSION in $PLAYER_YAML"
exit 0