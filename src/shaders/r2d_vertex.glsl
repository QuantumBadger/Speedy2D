#version 110

/*
 *  Copyright 2021 QuantumBadger
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

attribute vec2 in_Position;
attribute vec4 in_Color;
attribute vec2 in_TextureCoord;
attribute float in_TextureMix;
attribute float in_CircleMix;

uniform float in_ScaleX;
uniform float in_ScaleY;

varying vec4 pass_Color;
varying vec2 pass_TextureCoord;
varying float pass_TextureMix;
varying float pass_CircleMix;

void main(void) {

    gl_Position = vec4(
            in_Position.x * in_ScaleX - 1.0,
            in_Position.y * in_ScaleY + 1.0,
            0.0,
            1.0);

    pass_Color = in_Color;
    pass_TextureCoord = in_TextureCoord;
    pass_TextureMix = in_TextureMix;
    pass_CircleMix = in_CircleMix;
}