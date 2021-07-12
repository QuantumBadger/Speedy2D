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

uniform sampler2D in_Texture;

varying vec4 pass_Color;
varying vec2 pass_TextureCoord;
varying float pass_TextureMix;
varying float pass_CircleMix;

void main(void) {

    vec4 texCol = texture2D(in_Texture, pass_TextureCoord);

    float texCoordMagSquared = pass_TextureCoord.x * pass_TextureCoord.x
            + pass_TextureCoord.y * pass_TextureCoord.y;

    float circleAlpha = 1.0 - step(1.0, texCoordMagSquared);

    gl_FragColor = pass_Color * (
            vec4(1.0 - pass_TextureMix - pass_CircleMix)
                    + (texCol * pass_TextureMix)
                    + (vec4(vec3(1.0), circleAlpha)) * pass_CircleMix);
}